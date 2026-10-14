import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';
import 'support/migration_fixture.dart';

void main() {
  late MigrationFixture fixture;
  setUp(() => fixture = MigrationFixture());
  tearDown(() => fixture.dispose());

  test(
    'an explicitly empty bundle is managed and requires writable startup',
    () async {
      expectOk(await fixture.open(migrations: {}).close());
      expect(
        fixture.read('SELECT name FROM __dust_schema_migrations'),
        isEmpty,
      );
      expect(
        () => Sqlite3Driver.connect(
          SqliteConnectOptions.readOnly(fixture.path),
          migrations: {},
        ),
        throwsA(
          isA<Exception>().having(
            (e) => e.toString(),
            'message',
            contains('read-only connections cannot apply Dust migrations'),
          ),
        ),
      );
    },
  );

  test(
    'exact UTF-8 SQL and quoted names have an independently known hash',
    () async {
      final db = fixture.open(
        migrations: {"0001_'🍜.sql": 'CREATE TABLE "café" (label TEXT);'},
      );
      expectOk(await db.close());
      expect(
        fixture.read('SELECT name, checksum FROM __dust_schema_migrations'),
        [
          [
            "0001_'🍜.sql",
            'faabd5c20f6d4add0e00203c9ef94907a787571f73761a96bceaa08790f40179',
          ],
        ],
      );
    },
  );

  test(
    'exact checksums survive reopen; down files and map order do not run',
    () async {
      final migrations = {
        '0002.sql': upgradeItems,
        '0001.sql': createItems,
        '0003.down.sql': 'DROP TABLE items;',
      };
      final first = fixture.open(migrations: migrations);
      expectOk(
        await first.execute('INSERT INTO items (name) VALUES (?)', ['Ada']),
      );
      expectOk(await first.close());
      final reopened = fixture.open(migrations: migrations);
      expect(
        expectOk(
          await reopened.fetchScalar<String>('SELECT name FROM items', []),
        ),
        'Ada',
      );
      expectOk(await reopened.close());
      expect(
        fixture.read(
          'SELECT name, checksum, checksum_source FROM __dust_schema_migrations ORDER BY name',
        ),
        [
          ['0001.sql', createChecksum, 'applied'],
          ['0002.sql', upgradeChecksum, 'applied'],
        ],
      );
    },
  );

  test(
    'whitespace drift fails before pending SQL or history changes',
    () async {
      expectOk(await fixture.open(migrations: firstMigration).close());
      fixture.expectError(
        {'0001.sql': '$createItems ', '0002.sql': upgradeItems},
        '0001.sql',
        'SQLite migration `0001.sql` checksum mismatch.',
      );
      expect(fixture.read('SELECT checksum FROM __dust_schema_migrations'), [
        [createChecksum],
      ]);
      expect(fixture.read('PRAGMA table_info(items)').map((row) => row[1]), [
        'id',
        'name',
      ]);
    },
  );

  for (final empty in [false, true]) {
    test('missing applied migration fails, empty bundle: $empty', () async {
      expectOk(await fixture.open(migrations: firstMigration).close());
      fixture.expectError(
        empty ? {} : {'0002.sql': upgradeItems},
        '0001.sql',
        'SQLite migration `0001.sql` is missing from the supplied bundle.',
      );
      expect(fixture.read('PRAGMA table_info(items)').map((row) => row[1]), [
        'id',
        'name',
      ]);
    });
  }

  test(
    'omitting migration management leaves legacy history untouched',
    () async {
      fixture.seedLegacy();
      final db = fixture.open();
      expect(
        expectOk(await db.fetchScalar<String>('SELECT name FROM items', [])),
        'Ada 🍜',
      );
      expectOk(await db.close());
      final readonly = Sqlite3Driver.connect(
        SqliteConnectOptions.readOnly(fixture.path),
      );
      try {
        expect(
          expectOk(
            await readonly.fetchScalar<int>('SELECT COUNT(*) FROM items', []),
          ),
          1,
        );
      } finally {
        expectOk(await readonly.close());
      }
      expect(
        fixture
            .read('PRAGMA table_info(__dust_schema_migrations)')
            .map((row) => row[1]),
        ['name', 'applied_at'],
      );
    },
  );
}
