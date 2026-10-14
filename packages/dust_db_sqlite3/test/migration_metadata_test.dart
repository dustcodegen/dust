import 'package:dust_dart/db.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';
import 'support/migration_fixture.dart';

void main() {
  late MigrationFixture fixture;
  setUp(() => fixture = MigrationFixture());
  tearDown(() => fixture.dispose());

  for (final baseline in [false, true]) {
    test('missing applied checksum cannot be rebaselined: $baseline', () async {
      expectOk(await fixture.open(migrations: firstMigration).close());
      fixture.prepare('UPDATE __dust_schema_migrations SET checksum = NULL;');
      fixture.expectError(
        upgradedMigrations,
        '0001.sql',
        'SQLite migration `0001.sql` checksum is missing.',
        baseline: baseline,
      );
      expect(fixture.read('PRAGMA table_info(items)').map((row) => row[1]), [
        'id',
        'name',
      ]);
    });
    test('unknown checksum provenance is rejected: $baseline', () async {
      expectOk(await fixture.open(migrations: firstMigration).close());
      fixture.prepare(
        "UPDATE __dust_schema_migrations SET checksum_source = 'unknown';",
      );
      fixture.expectError(
        upgradedMigrations,
        '0001.sql',
        'SQLite migration `0001.sql` checksum provenance is invalid.',
        baseline: baseline,
      );
      expect(fixture.read('PRAGMA table_info(items)').map((row) => row[1]), [
        'id',
        'name',
      ]);
    });
  }

  test(
    'an existing checksum is never overwritten by explicit baseline',
    () async {
      expectOk(await fixture.open(migrations: firstMigration).close());
      fixture.prepare(
        "UPDATE __dust_schema_migrations SET checksum = 'broken';",
      );
      fixture.expectError(
        firstMigration,
        '0001.sql',
        'SQLite migration `0001.sql` checksum mismatch.',
        baseline: true,
      );
      expect(fixture.read('SELECT checksum FROM __dust_schema_migrations'), [
        ['broken'],
      ]);
    },
  );

  test('duplicate history is rejected rather than silently collapsed', () {
    fixture.prepare('''
$createItems
CREATE TABLE __dust_schema_migrations (
  name TEXT, applied_at TEXT, checksum TEXT, checksum_source TEXT
);
INSERT INTO __dust_schema_migrations VALUES
  ('0001.sql', '$legacyTimestamp', '$createChecksum', 'applied'),
  ('0001.sql', '$legacyTimestamp', '$createChecksum', 'applied');
''');
    fixture.expectError(
      firstMigration,
      '__dust_schema_migrations',
      'SQLite migration history read failed.',
    );
    expect(fixture.read('SELECT name FROM __dust_schema_migrations'), [
      ['0001.sql'],
      ['0001.sql'],
    ]);
  });

  test('invalid native history types retain a typed migration error', () {
    fixture.prepare('''
CREATE TABLE __dust_schema_migrations (
  name TEXT PRIMARY KEY, applied_at TEXT, checksum TEXT, checksum_source TEXT
);
INSERT INTO __dust_schema_migrations VALUES ('0001.sql', '$legacyTimestamp', '$createChecksum', NULL);
''');
    expect(
      () => fixture.open(migrations: firstMigration),
      throwsA(
        isA<SqlxError>()
            .having((e) => e.category, 'category', SqlxErrorCategory.migration)
            .having((e) => e.operation, 'operation', '__dust_schema_migrations')
            .having((e) => e.cause, 'cause', isA<TypeError>()),
      ),
    );
  });

  test(
    'mixed legacy and verified history fails atomically on later drift',
    () async {
      expectOk(await fixture.open(migrations: upgradedMigrations).close());
      fixture.prepare(
        "UPDATE __dust_schema_migrations SET checksum = NULL, checksum_source = 'legacy-baseline' WHERE name = '0001.sql';",
      );
      fixture.expectError(
        {'0001.sql': createItems, '0002.sql': '$upgradeItems '},
        '0002.sql',
        'SQLite migration `0002.sql` checksum mismatch.',
        baseline: true,
      );
      expect(
        fixture.read(
          'SELECT checksum FROM __dust_schema_migrations ORDER BY name',
        ),
        [
          [null],
          [upgradeChecksum],
        ],
      );
    },
  );
}
