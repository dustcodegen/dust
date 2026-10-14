import 'dart:async';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show initializeSqlite3Connection;
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  for (final failure in [
    'returned error',
    'thrown callback',
    'failed commit',
  ]) {
    test('$failure and failed rollback reject subsequent work', () async {
      final native = _FailingRollbackDatabase();
      native.inner.execute('''
CREATE TABLE parents (id INTEGER PRIMARY KEY);
CREATE TABLE children (
  id INTEGER REFERENCES parents(id) DEFERRABLE INITIALLY DEFERRED
);
''');
      final db = initializeSqlite3Connection(
        native,
        const SqliteConnectOptions.memory(foreignKeys: true),
        null,
      );
      addTearDown(() async => expectOk(await db.close()));
      final entered = Completer<void>();
      final release = Completer<void>();
      final owner = db.transaction<Unit>((tx) async {
        expectOk(await tx.execute('INSERT INTO children VALUES (1)', []));
        entered.complete();
        await release.future;
        if (failure == 'thrown callback') throw StateError('abort');
        if (failure == 'failed commit') return const Ok(unit);
        return Err(SqlxError.transaction('abort'));
      });
      await entered.future;
      release.complete();
      expect((await owner).unwrapErr().operation, 'ROLLBACK');
      expect(native.inner.autocommit, isFalse);
      for (final result in [
        await db.execute('INSERT INTO parents VALUES (2)', []),
        await db.fetchScalar<int>('SELECT COUNT(*) FROM parents', []),
        await db.transaction<Unit>((_) async => const Ok(unit)),
      ]) {
        expect(result.unwrapErr().message, contains('Close and reopen'));
      }
      expect(native.inner.select('SELECT * FROM parents'), isEmpty);
      expectOk(await db.close());
      expect(
        (await db.fetchScalar<int>('SELECT 1', [])).unwrapErr().operation,
        'checkOpen',
      );
    });
  }

  test('failed savepoint cleanup cannot be ignored and committed', () async {
    final native = _FailingRollbackDatabase();
    native.inner.execute('CREATE TABLE entries (id INTEGER PRIMARY KEY)');
    final db = initializeSqlite3Connection(
      native,
      const SqliteConnectOptions.memory(),
      null,
    );
    addTearDown(() async => expectOk(await db.close()));
    late Result<ExecResult, SqlxError> parentWrite;
    final result = await db.transaction<Unit>((tx) async {
      final nested = await tx.transaction<Unit>((child) async {
        expectOk(await child.execute('INSERT INTO entries VALUES (1)', []));
        return Err(SqlxError.transaction('abort child'));
      });
      expect(nested.isErr, isTrue);
      parentWrite = await tx.execute('INSERT INTO entries VALUES (2)', []);
      return const Ok(unit); // Ignoring the nested error must not commit it.
    });
    expect(parentWrite.isErr, isTrue);
    expect(result.isErr, isTrue);
    expect(native.inner.autocommit, isFalse);
    expect((await db.fetchScalar<int>('SELECT 1', [])).isErr, isTrue);
  });

  test(
    'failed release forces root rollback even when nested error is ignored',
    () async {
      final native = _FailingRollbackDatabase(
        failRollback: false,
        failRelease: true,
      );
      native.inner.execute('CREATE TABLE entries (id INTEGER PRIMARY KEY)');
      final db = initializeSqlite3Connection(
        native,
        const SqliteConnectOptions.memory(),
        null,
      );
      addTearDown(db.close);
      final result = await db.transaction<Unit>((tx) async {
        final child = await tx.transaction<Unit>(
          (inner) async => (await inner.execute(
            'INSERT INTO entries VALUES (1)',
            [],
          )).map((_) => unit),
        );
        expect(child.unwrapErr().operation, 'RELEASE SAVEPOINT _dust_tx_1');
        return const Ok(unit);
      });
      expect(result.unwrapErr().operation, 'RELEASE SAVEPOINT _dust_tx_1');
      expect(native.inner.autocommit, isTrue);
      expect(native.inner.select('SELECT * FROM entries'), isEmpty);
      expect(
        (await db.fetchScalar<int>('SELECT 1', [])).unwrapErr().message,
        contains('Close and reopen'),
      );
      expectOk(await db.close());
    },
  );
}

/// Real SQLite executes everything except the injected cleanup failures.
final class _FailingRollbackDatabase implements sqlite.Database {
  _FailingRollbackDatabase({
    this.failRollback = true,
    this.failRelease = false,
  });

  final bool failRollback;
  final bool failRelease;
  final sqlite.Database inner = sqlite.sqlite3.openInMemory();

  @override
  bool get autocommit => inner.autocommit;

  @override
  int get updatedRows => inner.updatedRows;

  @override
  int get lastInsertRowId => inner.lastInsertRowId;

  @override
  void execute(String sql, [List<Object?> parameters = const []]) {
    if (failRollback && sql.startsWith('ROLLBACK')) {
      throw StateError('injected rollback failure');
    }
    if (failRelease && sql.startsWith('RELEASE')) {
      throw StateError('injected release failure');
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
    if (failRollback && sql == 'ROLLBACK') {
      throw StateError('injected rollback failure');
    }
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
