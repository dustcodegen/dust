import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show initializeSqlite3Connection;
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

import 'support/expect_ok.dart';
import 'support/migration_fixture.dart';

void main() {
  for (final failureAt in [
    'PRAGMA table_info',
    'ADD COLUMN checksum TEXT',
    'ADD COLUMN checksum_source',
  ]) {
    test(
      'failed history upgrade at $failureAt rolls back and closes',
      () async {
        final fixture = MigrationFixture()..seedLegacy();
        addTearDown(fixture.dispose);
        final native = _FailedUpgrade(
          sqlite.sqlite3.open(fixture.path),
          failureAt,
        );
        addTearDown(() {
          if (native.closeAttempts == 0) native.close();
        });
        expect(
          () => initializeSqlite3Connection(
            native,
            SqliteConnectOptions.path(
              fixture.path,
              baselineLegacyMigrations: true,
            ),
            firstMigration,
          ),
          throwsA(
            isA<SqlxError>()
                .having(
                  (e) => e.category,
                  'category',
                  SqlxErrorCategory.migration,
                )
                .having(
                  (e) => e.operation,
                  'operation',
                  '__dust_schema_migrations',
                )
                .having(
                  (e) => e.message,
                  'message',
                  'SQLite migration table setup failed.',
                )
                .having((e) => e.cause, 'cause', same(native.failure)),
          ),
        );
        expect(native.closeAttempts, 1);
        expect(() => native.inner.select('SELECT 1'), throwsStateError);
        expect(
          fixture
              .read('PRAGMA table_info(__dust_schema_migrations)')
              .map((row) => row[1]),
          ['name', 'applied_at'],
        );
        expect(fixture.read('SELECT name FROM items'), [
          ['Ada 🍜'],
        ]);
        expectOk(
          await fixture
              .open(migrations: firstMigration, baseline: true)
              .close(),
        );
      },
    );
  }
}

/// Real SQLite except for the precise injected metadata-upgrade failure.
final class _FailedUpgrade implements sqlite.Database {
  _FailedUpgrade(this.inner, this.failureAt);
  final sqlite.Database inner;
  final String failureAt;
  final failure = StateError('native history upgrade failed');
  var closeAttempts = 0;

  @override
  bool get autocommit => inner.autocommit;

  @override
  sqlite.VoidPredicate? get commitFilter => inner.commitFilter;

  @override
  set commitFilter(sqlite.VoidPredicate? value) => inner.commitFilter = value;

  @override
  void execute(String sql, [List<Object?> parameters = const []]) {
    if (sql.contains(failureAt)) throw failure;
    inner.execute(sql, parameters);
  }

  @override
  sqlite.ResultSet select(String sql, [List<Object?> parameters = const []]) {
    if (sql.contains(failureAt)) throw failure;
    return inner.select(sql, parameters);
  }

  @override
  void close() {
    closeAttempts++;
    inner.close();
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'Unexpected native operation: ${invocation.memberName}',
  );
}
