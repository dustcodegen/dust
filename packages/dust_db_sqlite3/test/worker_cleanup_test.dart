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

void _cleanupFailureWorker(List<Object?> arguments) => runSqlite3Worker(
  arguments,
  open: (options, _) {
    final native = _FailingCleanupDatabase(sqlite.sqlite3.open(options.path!));
    native.inner.execute('CREATE TABLE entries (id INTEGER PRIMARY KEY)');
    return initializeSqlite3Connection(native, options, null);
  },
);

void main() {
  for (final nested in [false, true]) {
    test('failed cleanup poisons worker, nested: $nested', () async {
      final fixture = MigrationFixture();
      addTearDown(fixture.dispose);
      final db = await openSqlite3WorkerConnection(
        SqliteConnectOptions.path(fixture.path),
        null,
        entrypoint: _cleanupFailureWorker,
      );
      addTearDown(db.close);
      final result = await db.transaction<Unit>((tx) async {
        if (nested) {
          final child = await tx.transaction<Unit>((child) async {
            expectOk(await child.execute('INSERT INTO entries VALUES (1)', []));
            return Err(SqlxError.transaction('abort child'));
          });
          expect(child.unwrapErr().operation, startsWith('ROLLBACK'));
          expect(
            (await tx.execute('INSERT INTO entries VALUES (2)', [])).isErr,
            isTrue,
          );
          return const Ok(unit); // Ignoring cleanup failure must not commit.
        }
        expectOk(await tx.execute('INSERT INTO entries VALUES (1)', []));
        return Err(SqlxError.transaction('abort root'));
      });
      expect(result.unwrapErr().operation, 'ROLLBACK');
      expect(result.unwrapErr().cause, contains('injected rollback failure'));
      expect(
        (await db.fetchScalar<int>('SELECT 1', [])).unwrapErr().message,
        contains('Close and reopen'),
      );
      expect(
        (await db.transaction<Unit>((_) async => const Ok(unit))).isErr,
        isTrue,
      );
      expectOk(await db.close());
      final reopened = await Sqlite3WorkerDriver.open(fixture.path);
      try {
        expect(
          expectOk(
            await reopened.fetchScalar<int>('SELECT COUNT(*) FROM entries', []),
          ),
          0,
        );
        expectOk(await reopened.execute('INSERT INTO entries VALUES (3)', []));
      } finally {
        expectOk(await reopened.close());
      }
    });
  }

  test(
    'foreground and worker connections choose execution independently',
    () async {
      final foreground = Sqlite3Driver.connect(
        const SqliteConnectOptions.memory(),
      );
      final background = await Sqlite3WorkerDriver.connect(
        const SqliteConnectOptions.memory(),
      );
      try {
        final result = await background.transaction<Unit>((tx) async {
          expectOk(
            await foreground.execute('CREATE TABLE entries (id INTEGER)', []),
          );
          expectOk(await tx.execute('CREATE TABLE entries (id INTEGER)', []));
          expectOk(
            await foreground.execute('INSERT INTO entries VALUES (1)', []),
          );
          expect(
            expectOk(
              await tx.fetchScalar<int>('SELECT COUNT(*) FROM entries', []),
            ),
            0,
          );
          return const Ok(unit);
        });
        expectOk(result);
        expect(
          expectOk(
            await foreground.fetchScalar<int>(
              'SELECT COUNT(*) FROM entries',
              [],
            ),
          ),
          1,
        );
      } finally {
        expectOk(await background.close());
        expectOk(await foreground.close());
      }
    },
  );
}

/// Owns real SQLite; only rollback is deliberately broken inside the worker.
final class _FailingCleanupDatabase implements sqlite.Database {
  _FailingCleanupDatabase(this.inner);
  final sqlite.Database inner;

  @override
  bool get autocommit => inner.autocommit;
  @override
  int get updatedRows => inner.updatedRows;
  @override
  int get lastInsertRowId => inner.lastInsertRowId;
  @override
  void execute(String sql, [List<Object?> parameters = const []]) {
    if (sql.startsWith('ROLLBACK')) {
      throw StateError('injected rollback failure');
    }
    inner.execute(sql, parameters);
  }

  @override
  sqlite.PreparedStatement prepare(
    String sql, {
    bool persistent = false,
    bool vtab = true,
    bool checkNoTail = false,
  }) {
    if (sql == 'ROLLBACK') throw StateError('injected rollback failure');
    return inner.prepare(
      sql,
      persistent: persistent,
      vtab: vtab,
      checkNoTail: checkNoTail,
    );
  }

  @override
  void close() => inner.close();
  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'Unexpected native operation: ${invocation.memberName}',
  );
}
