import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show
        initializeSqlite3Connection,
        openSqlite3WorkerConnection,
        runSqlite3Worker;
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

import 'support/expect_ok.dart';
import 'support/migration_fixture.dart';

void _legacyMemoryWorker(List<Object?> arguments) => runSqlite3Worker(
  arguments,
  open: (options, migrations) {
    final native = sqlite.sqlite3.openInMemory();
    native.execute('''
$createItems
INSERT INTO items VALUES (1, 'Ada 🍜');
CREATE TABLE __dust_schema_migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL);
INSERT INTO __dust_schema_migrations VALUES ('0001.sql', '$legacyTimestamp');
''');
    return initializeSqlite3Connection(native, options, migrations);
  },
);

Matcher migrationError(String name, String message) => isA<SqlxError>()
    .having((e) => e.category, 'category', SqlxErrorCategory.migration)
    .having((e) => e.driver, 'driver', Driver.sqlite3)
    .having((e) => e.operation, 'operation', name)
    .having((e) => e.message, 'message', message);

void main() {
  late MigrationFixture fixture;
  setUp(() => fixture = MigrationFixture());
  tearDown(() => fixture.dispose());

  test(
    'worker records exact checksums and rejects changed, missing and empty bundles',
    () async {
      final first = await Sqlite3WorkerDriver.open(
        fixture.path,
        migrations: firstMigration,
      );
      expectOk(await first.close());
      expect(
        fixture.read(
          'SELECT checksum, checksum_source FROM __dust_schema_migrations',
        ),
        [
          [createChecksum, 'applied'],
        ],
      );
      await expectLater(
        Sqlite3WorkerDriver.open(
          fixture.path,
          migrations: {'0001.sql': '$createItems ', '0002.sql': upgradeItems},
        ),
        throwsA(
          migrationError(
            '0001.sql',
            'SQLite migration `0001.sql` checksum mismatch.',
          ),
        ),
      );
      for (final bundle in [
        <String, String>{},
        {'0002.sql': upgradeItems},
      ]) {
        await expectLater(
          Sqlite3WorkerDriver.open(fixture.path, migrations: bundle),
          throwsA(
            migrationError(
              '0001.sql',
              'SQLite migration `0001.sql` is missing from the supplied bundle.',
            ),
          ),
        );
      }
      expect(fixture.read('PRAGMA table_info(items)').map((row) => row[1]), [
        'id',
        'name',
      ]);
      final reopened = await Sqlite3WorkerDriver.open(
        fixture.path,
        migrations: firstMigration,
      );
      expectOk(await reopened.close());
    },
  );

  test(
    'legacy worker startup requires explicit baseline and preserves provenance',
    () async {
      fixture.seedLegacy();
      await expectLater(
        Sqlite3WorkerDriver.open(fixture.path, migrations: upgradedMigrations),
        throwsA(
          migrationError(
            '0001.sql',
            'SQLite migration `0001.sql` has unverified legacy history. Review the deployed SQL and explicitly enable baselineLegacyMigrations.',
          ),
        ),
      );
      expect(
        fixture
            .read('PRAGMA table_info(__dust_schema_migrations)')
            .map((row) => row[1]),
        ['name', 'applied_at'],
      );
      final db = await Sqlite3WorkerDriver.open(
        fixture.path,
        migrations: upgradedMigrations,
        options: const SqliteConnectOptions(baselineLegacyMigrations: true),
      );
      try {
        expect(
          expectOk(await db.fetchScalar<String>('SELECT name FROM items', [])),
          'Ada 🍜',
        );
      } finally {
        expectOk(await db.close());
      }
      expect(
        fixture.read(
          'SELECT checksum_source FROM __dust_schema_migrations ORDER BY name',
        ),
        [
          ['legacy-baseline'],
          ['applied'],
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
      final reopened = await Sqlite3WorkerDriver.open(
        fixture.path,
        migrations: upgradedMigrations,
      );
      expectOk(await reopened.close());
    },
  );

  test(
    'legacy opt-in also survives the memory-options protocol branch',
    () async {
      final db = await openSqlite3WorkerConnection(
        const SqliteConnectOptions.memory(baselineLegacyMigrations: true),
        firstMigration,
        entrypoint: _legacyMemoryWorker,
      );
      try {
        expect(
          expectOk(
            await db.fetchScalar<String>(
              'SELECT checksum_source FROM __dust_schema_migrations',
              [],
            ),
          ),
          'legacy-baseline',
        );
      } finally {
        expectOk(await db.close());
      }
    },
  );

  test(
    'omitted migrations allow read-only inspection without altering legacy history',
    () async {
      fixture.seedLegacy();
      final db = await Sqlite3WorkerDriver.connect(
        SqliteConnectOptions.readOnly(fixture.path),
      );
      try {
        expect(
          expectOk(await db.fetchScalar<int>('SELECT COUNT(*) FROM items', [])),
          1,
        );
        expect(
          (await db.execute('DELETE FROM items', [])).unwrapErr().category,
          SqlxErrorCategory.query,
        );
      } finally {
        expectOk(await db.close());
      }
      expect(
        fixture
            .read('PRAGMA table_info(__dust_schema_migrations)')
            .map((row) => row[1]),
        ['name', 'applied_at'],
      );
      await expectLater(
        Sqlite3WorkerDriver.connect(
          SqliteConnectOptions.readOnly(fixture.path),
          migrations: {},
        ),
        throwsA(isA<SqlxError>()),
      );
    },
  );

  test(
    'an explicitly empty worker bundle is managed on a fresh database',
    () async {
      final db = await Sqlite3WorkerDriver.open(fixture.path, migrations: {});
      expectOk(await db.close());
      expect(
        fixture.read('SELECT name FROM __dust_schema_migrations'),
        isEmpty,
      );
    },
  );

  test(
    'failed pending SQL rolls back the legacy upgrade and closes its native handle',
    () async {
      fixture.seedLegacy();
      await expectLater(
        Sqlite3WorkerDriver.open(
          fixture.path,
          options: const SqliteConnectOptions(baselineLegacyMigrations: true),
          migrations: {
            '0001.sql': createItems,
            '0002.sql': '$upgradeItems INSERT INTO missing VALUES (1);',
          },
        ),
        throwsA(
          isA<SqlxError>().having((e) => e.operation, 'operation', '0002.sql'),
        ),
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
    },
  );
}
