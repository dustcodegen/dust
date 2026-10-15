import 'dart:async';
import 'dart:isolate';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show openSqlite3WorkerConnection;
import 'package:test/test.dart';

void _exitBeforeReady(List<Object?> _) => Isolate.exit();
void _errorBeforeReady(List<Object?> _) => throw StateError('worker startup');

void _malformedStartup(List<Object?> arguments) =>
    (arguments[0]! as SendPort).send('not a reply');

void _unknownReply(List<Object?> arguments) {
  final replies = arguments[0]! as SendPort;
  final commands = ReceivePort();
  commands.listen((_) => replies.send([-1, true, null]));
  replies.send([0, true, commands.sendPort]);
}

void _exitOnRequest(List<Object?> arguments) {
  final replies = arguments[0]! as SendPort;
  final commands = ReceivePort();
  commands.listen((_) => Isolate.exit());
  replies.send([0, true, commands.sendPort]);
}

void _exitInTransaction(List<Object?> arguments) {
  final replies = arguments[0]! as SendPort;
  final commands = ReceivePort();
  commands.listen((message) {
    final request = message! as List<Object?>;
    if (request[1] == 'begin') {
      replies.send([request[0], true, request[0]]);
      Timer.run(Isolate.exit);
    } else {
      Isolate.exit();
    }
  });
  replies.send([0, true, commands.sendPort]);
}

void main() {
  test(
    'malformed startup rejects rather than stranding the ready Future',
    () async {
      await expectLater(
        openSqlite3WorkerConnection(
          const SqliteConnectOptions.memory(),
          null,
          entrypoint: _malformedStartup,
        ),
        throwsA(
          isA<SqlxError>().having(
            (e) => e.operation,
            'operation',
            'worker protocol',
          ),
        ),
      );
    },
  );

  test(
    'unknown reply settles pending requests and makes the worker terminal',
    () async {
      final db = await openSqlite3WorkerConnection(
        const SqliteConnectOptions.memory(),
        null,
        entrypoint: _unknownReply,
      );
      final one = db.fetchScalar<int>('SELECT 1', []);
      final two = db.fetchScalar<int>('SELECT 2', []);
      expect((await one).unwrapErr().operation, 'worker protocol');
      expect((await two).unwrapErr().operation, 'worker protocol');
      expect((await db.close()).unwrapErr().operation, 'worker protocol');
    },
  );
  test('send failure settles request and terminal connection', () async {
    final db = await openSqlite3WorkerConnection(
      const SqliteConnectOptions.memory(),
      {},
      entrypoint: _exitOnRequest,
      send: (_, _) => throw StateError('send failed'),
    );
    expect(
      (await db.fetchScalar<int>('SELECT 1', [])).unwrapErr().operation,
      'worker send',
    );
    expect((await db.close()).unwrapErr().operation, 'worker send');
  });
  for (final entrypoint in [_exitBeforeReady, _errorBeforeReady]) {
    test(
      'terminal startup ${entrypoint == _exitBeforeReady ? 'exit' : 'error'} settles ready future',
      () async {
        await expectLater(
          openSqlite3WorkerConnection(
            const SqliteConnectOptions.memory(),
            {},
            entrypoint: entrypoint,
          ),
          throwsA(
            isA<SqlxError>().having(
              (e) => e.operation,
              'operation',
              'worker exit',
            ),
          ),
        );
      },
    );
  }

  test(
    'worker exit settles active and pending requests and remains terminal',
    () async {
      final db = await openSqlite3WorkerConnection(
        const SqliteConnectOptions.memory(),
        {},
        entrypoint: _exitOnRequest,
      );
      final first = db.fetchScalar<int>('SELECT 1', []);
      final queued = db.execute('SELECT 2', []);
      expect((await first).unwrapErr().operation, 'worker exit');
      expect((await queued).unwrapErr().operation, 'worker exit');
      expect(
        (await db.fetchScalar<int>('SELECT 3', [])).unwrapErr().operation,
        'worker exit',
      );
      expect((await db.close()).unwrapErr().operation, 'worker exit');
      expect((await db.close()).unwrapErr().operation, 'worker exit');
    },
  );

  test('worker exit aborts a callback waiting on external I/O', () async {
    final db = await openSqlite3WorkerConnection(
      const SqliteConnectOptions.memory(),
      {},
      entrypoint: _exitInTransaction,
    );
    final entered = Completer<Transaction>();
    final release = Completer<void>();
    final transaction = db.transaction<Unit>((tx) async {
      entered.complete(tx);
      await release.future;
      return const Ok(unit);
    });
    final tx = await entered.future;
    // Capturing an executor from outside its callback is rejected before RPC.
    expect(
      (await tx.execute('SELECT 1', [])).unwrapErr().category,
      SqlxErrorCategory.transaction,
    );
    // The worker exits independently while the caller awaits external I/O.
    try {
      expect((await transaction).unwrapErr().operation, 'worker exit');
    } finally {
      release.complete();
    }
    expect((await db.close()).unwrapErr().operation, 'worker exit');
  });

  test('spawn failure from an unsendable entrypoint settles startup', () async {
    final port = ReceivePort();
    addTearDown(port.close);
    await expectLater(
      openSqlite3WorkerConnection(
        const SqliteConnectOptions.memory(),
        {},
        entrypoint: (_) {
          port.close();
        },
      ),
      throwsA(
        isA<SqlxError>().having(
          (e) => e.operation,
          'operation',
          'worker spawn',
        ),
      ),
    );
  });
}
