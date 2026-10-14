import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show initializeSqlite3Connection;
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

import 'support/expect_ok.dart';
import 'support/migration_fixture.dart';

void main() {
  test(
    'failed rollback retains cleanup cause and closes the active handle',
    () async {
      final fixture = MigrationFixture();
      addTearDown(fixture.dispose);
      expectOk(await fixture.open(migrations: firstMigration).close());
      final native = _FailedRollback(sqlite.sqlite3.open(fixture.path));
      addTearDown(() {
        if (native.closeAttempts == 0) native.close();
      });
      expect(
        () => initializeSqlite3Connection(
          native,
          SqliteConnectOptions.path(fixture.path),
          {
            '0001.sql': createItems,
            '0002.sql':
                "INSERT INTO items VALUES (1, 'pending'); THIS IS NOT SQL;",
          },
        ),
        throwsA(
          isA<SqlxError>()
              .having(
                (e) => e.category,
                'category',
                SqlxErrorCategory.migration,
              )
              .having((e) => e.operation, 'operation', 'ROLLBACK')
              .having(
                (e) => e.message,
                'message',
                'SQLite migration rollback failed.',
              )
              .having((e) => e.cause, 'cause', same(native.failure)),
        ),
      );
      expect(native.closeAttempts, 1);
      expect(fixture.read('SELECT * FROM items'), isEmpty);
      expectOk(await fixture.open(migrations: upgradedMigrations).close());
    },
  );
  for (final escape in [
    'COMMIT;',
    'END TRANSACTION;',
    'ROLLBACK;',
    'ROLLBACK; BEGIN;',
  ]) {
    test('migration $escape cannot persist SQL outside owned startup', () async {
      final fixture = MigrationFixture();
      addTearDown(fixture.dispose);
      expectOk(await fixture.open(migrations: firstMigration).close());
      expect(
        () => fixture.open(
          migrations: {
            '0001.sql': createItems,
            '0002.sql':
                "INSERT INTO items VALUES (1, 'before'); $escape INSERT INTO items VALUES (2, 'after');",
          },
        ),
        throwsA(
          isA<SqlxError>().having(
            (e) => e.category,
            'category',
            SqlxErrorCategory.migration,
          ),
        ),
      );
      expect(fixture.read('SELECT * FROM items'), isEmpty);
      expect(fixture.read('SELECT name FROM __dust_schema_migrations'), [
        ['0001.sql'],
      ]);
      expectOk(await fixture.open(migrations: upgradedMigrations).close());
    });
  }

  for (final permit in [true, false]) {
    test('an existing native commit filter is preserved: $permit', () async {
      final native = sqlite.sqlite3.openInMemory();
      var calls = 0;
      bool filter() {
        calls++;
        return permit;
      }

      native.commitFilter = filter;
      if (permit) {
        final db = initializeSqlite3Connection(
          native,
          const SqliteConnectOptions.memory(),
          firstMigration,
        );
        try {
          expect(native.commitFilter, same(filter));
          expect(calls, 1);
        } finally {
          expectOk(await db.close());
        }
      } else {
        expect(
          () => initializeSqlite3Connection(
            native,
            const SqliteConnectOptions.memory(),
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
                  (e) =>
                      (e.cause! as sqlite.SqliteException).extendedResultCode,
                  'commit veto',
                  531,
                ),
          ),
        );
        expect(calls, 1);
        expect(() => native.select('SELECT 1'), throwsStateError);
      }
    });
  }
}

/// Real SQLite whose still-active rollback is deliberately rejected.
final class _FailedRollback implements sqlite.Database {
  _FailedRollback(this.inner);
  final sqlite.Database inner;
  final failure = StateError('native migration rollback failed');
  var closeAttempts = 0;

  @override
  bool get autocommit => inner.autocommit;

  @override
  sqlite.VoidPredicate? get commitFilter => inner.commitFilter;

  @override
  set commitFilter(sqlite.VoidPredicate? value) => inner.commitFilter = value;

  @override
  void execute(String sql, [List<Object?> parameters = const []]) {
    if (sql == 'ROLLBACK') throw failure;
    inner.execute(sql, parameters);
  }

  @override
  sqlite.ResultSet select(String sql, [List<Object?> parameters = const []]) =>
      inner.select(sql, parameters);

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
