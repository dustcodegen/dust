import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';
import 'support/migration_fixture.dart';

void main() {
  late MigrationFixture fixture;
  setUp(() => fixture = MigrationFixture());
  tearDown(() => fixture.dispose());

  test('legacy history fails closed and rolls back the metadata upgrade', () {
    fixture.seedLegacy();
    fixture.expectError(
      upgradedMigrations,
      '0001.sql',
      'SQLite migration `0001.sql` has unverified legacy history. '
          'Review the deployed SQL and explicitly enable baselineLegacyMigrations.',
    );
    expect(
      fixture
          .read('PRAGMA table_info(__dust_schema_migrations)')
          .map((row) => row[1]),
      ['name', 'applied_at'],
    );
    expect(fixture.read('SELECT * FROM items'), [
      [1, 'Ada 🍜'],
    ]);
    expect(fixture.read('SELECT * FROM __dust_schema_migrations'), [
      ['0001.sql', legacyTimestamp],
    ]);
  });

  test(
    'explicit legacy baseline preserves data, timestamps and honest provenance',
    () async {
      fixture.seedLegacy();
      final db = fixture.open(migrations: upgradedMigrations, baseline: true);
      expectOk(await db.close());
      expect(fixture.read('SELECT * FROM items'), [
        [1, 'Ada 🍜', 1],
      ]);
      expect(
        fixture.read(
          'SELECT name, checksum, checksum_source FROM __dust_schema_migrations ORDER BY name',
        ),
        [
          ['0001.sql', createChecksum, 'legacy-baseline'],
          ['0002.sql', upgradeChecksum, 'applied'],
        ],
      );
      expect(
        fixture.read(
          "SELECT applied_at FROM __dust_schema_migrations WHERE name = '0001.sql'",
        ),
        [
          [legacyTimestamp],
        ],
      );
      expectOk(await fixture.open(migrations: upgradedMigrations).close());
      fixture.expectError(
        {'0001.sql': '$createItems ', '0002.sql': upgradeItems},
        '0001.sql',
        'SQLite migration `0001.sql` checksum mismatch.',
      );
    },
  );

  test(
    'a failed pending migration rolls back the legacy baseline and all SQL',
    () async {
      fixture.seedLegacy();
      expect(
        () => fixture.open(
          baseline: true,
          migrations: {
            '0001.sql': createItems,
            '0002.sql': '$upgradeItems INSERT INTO missing VALUES (1);',
          },
        ),
        throwsA(isA<Exception>()),
      );
      expect(
        fixture
            .read('PRAGMA table_info(__dust_schema_migrations)')
            .map((row) => row[1]),
        ['name', 'applied_at'],
      );
      expect(fixture.read('SELECT * FROM items'), [
        [1, 'Ada 🍜'],
      ]);
      expectOk(
        await fixture
            .open(migrations: upgradedMigrations, baseline: true)
            .close(),
      );
    },
  );

  test('explicit baseline never permits a missing applied migration', () {
    fixture.seedLegacy();
    expect(
      () =>
          fixture.open(migrations: {'0002.sql': upgradeItems}, baseline: true),
      throwsA(
        isA<Exception>().having(
          (e) => e.toString(),
          'message',
          contains('missing from the supplied bundle'),
        ),
      ),
    );
    expect(fixture.read('SELECT * FROM items'), [
      [1, 'Ada 🍜'],
    ]);
    expect(
      fixture
          .read('PRAGMA table_info(__dust_schema_migrations)')
          .map((row) => row[1]),
      ['name', 'applied_at'],
    );
  });

  test(
    'a baseline UPDATE failure rolls back the upgrade without changing data',
    () {
      fixture.seedLegacy();
      fixture.prepare('''
CREATE TRIGGER reject_baseline BEFORE UPDATE ON __dust_schema_migrations
BEGIN SELECT RAISE(ABORT, 'baseline denied'); END;
''');
      expect(
        () => fixture.open(migrations: upgradedMigrations, baseline: true),
        throwsA(
          isA<Exception>().having(
            (e) => e.toString(),
            'message',
            contains('SQLite migration checksum baseline failed.'),
          ),
        ),
      );
      expect(fixture.read('SELECT * FROM items'), [
        [1, 'Ada 🍜'],
      ]);
      expect(
        fixture
            .read('PRAGMA table_info(__dust_schema_migrations)')
            .map((row) => row[1]),
        ['name', 'applied_at'],
      );
    },
  );

  test('empty name-only history upgrades without requiring a baseline', () async {
    fixture.prepare(
      'CREATE TABLE __dust_schema_migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL);',
    );
    expectOk(await fixture.open(migrations: firstMigration).close());
    expect(
      fixture.read('SELECT checksum_source FROM __dust_schema_migrations'),
      [
        ['applied'],
      ],
    );
  });

  test('path and memory options carry the explicit baseline setting', () async {
    fixture.seedLegacy();
    final db = Sqlite3Driver.connect(
      SqliteConnectOptions.path(fixture.path, baselineLegacyMigrations: true),
      migrations: firstMigration,
    );
    expectOk(await db.close());
    expect(
      const SqliteConnectOptions.memory(
        baselineLegacyMigrations: true,
      ).baselineLegacyMigrations,
      isTrue,
    );
    expect(
      SqliteConnectOptions.readOnly(fixture.path).baselineLegacyMigrations,
      isFalse,
    );
  });
}
