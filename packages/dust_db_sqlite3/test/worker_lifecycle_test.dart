import 'dart:async';
import 'dart:isolate';

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

void _worker(List<Object?> arguments) => runSqlite3Worker(arguments);

void _failedCloseWorker(List<Object?> arguments) => runSqlite3Worker(
  arguments,
  open: (options, migrations) =>
      initializeSqlite3Connection(_FailedClose(), options, migrations),
);

final class _FailedClose implements sqlite.Database {
  @override
  void close() => throw StateError('native worker close failed');
  @override
  dynamic noSuchMethod(Invocation invocation) =>
      throw UnsupportedError('${invocation.memberName}');
}

void main() {
  test(
    'native worker close failure is retained without claiming release',
    () async {
      final db = await openSqlite3WorkerConnection(
        const SqliteConnectOptions.memory(),
        null,
        entrypoint: _failedCloseWorker,
      );
      final first = db.close();
      expect(db.close(), same(first));
      final error = (await first).unwrapErr();
      expect(error.category, SqlxErrorCategory.connection);
      expect(error.operation, 'close');
      expect(error.cause, 'Bad state: native worker close failed');
      expect((await db.close()).unwrapErr(), same(error));
      expect(
        (await db.execute('SELECT 1', [])).unwrapErr().category,
        SqlxErrorCategory.connection,
      );
    },
  );
  test(
    'failed BEGIN does not invoke callback or roll back existing SQL',
    () async {
      final db = await Sqlite3WorkerDriver.open(':memory:');
      addTearDown(db.close);
      expect(db.driver, Driver.sqlite3);
      expectOk(await db.execute('BEGIN', []));
      var called = false;
      final failed = await db.transaction<Unit>((_) async {
        called = true;
        return const Ok(unit);
      });
      expect(failed.unwrapErr().operation, 'BEGIN');
      expect(called, isFalse);
      expectOk(await db.execute('ROLLBACK', []));
      expectOk(await db.transaction<Unit>((_) async => const Ok(unit)));
    },
  );

  test(
    'closing scoped executor expires it without closing native owner',
    () async {
      final db = await Sqlite3WorkerDriver.open(':memory:');
      addTearDown(db.close);
      expectOk(
        await db.transaction<Unit>((tx) async {
          expectOk(await tx.close());
          expectOk(await tx.close());
          expect(
            (await tx.fetchScalar<int>('SELECT 1', [])).unwrapErr().category,
            SqlxErrorCategory.transaction,
          );
          return const Ok(unit);
        }),
      );
      expect(expectOk(await db.fetchScalar<int>('SELECT 2', [])), 2);
    },
  );

  test(
    'worker rejects an expired transaction token and closes its port',
    () async {
      final events = ReceivePort();
      final replies = StreamIterator<Object?>(events);
      final isolate = await Isolate.spawn(_worker, <Object?>[
        events.sendPort,
        <String, Object?>{
          'memory': true,
          'create': true,
          'readOnly': false,
          'pragmas': <String, Object>{},
        },
        <String, String>{},
      ]);
      try {
        expect(await replies.moveNext(), isTrue);
        final commands = (replies.current! as List)[2] as SendPort;
        commands.send([1, 'execute', 999]);
        expect(await replies.moveNext(), isTrue);
        expect(replies.current, [
          1,
          false,
          {
            'category': 'transaction',
            'message': 'SQLite transaction is closed.',
            'operation': 'execute',
            'kind': null,
            'cause': null,
          },
        ]);
        commands.send([2, 'close', null]);
        expect(await replies.moveNext(), isTrue);
        expect(replies.current, [2, true, null]);
      } finally {
        await replies.cancel();
        events.close();
        isolate.kill(priority: Isolate.immediate);
      }
    },
  );
}
