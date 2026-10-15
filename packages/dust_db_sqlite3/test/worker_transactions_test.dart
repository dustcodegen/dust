import 'dart:async';
import 'dart:isolate';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  late Sqlite3WorkerDriver db;
  setUp(
    () async => db = await Sqlite3WorkerDriver.open(
      ':memory:',
      migrations: const {
        '0001.sql': 'CREATE TABLE entries (id INTEGER PRIMARY KEY);',
      },
    ),
  );
  tearDown(() async => expectOk(await db.close()));

  test(
    'outside reads, writes, transactions and close fail fast across an await',
    () async {
      final entered = Completer<Transaction>();
      final release = Completer<void>();
      final aborted = SqlxError.transaction('abort');
      final owner = db.transaction<Unit>((tx) async {
        expectOk(await tx.execute('INSERT INTO entries VALUES (1)', []));
        entered.complete(tx);
        await release.future;
        return Err(aborted);
      });
      final escaped = await entered.future;
      var called = false;
      try {
        for (final result in await Future.wait([
          db.execute('INSERT INTO entries VALUES (2)', []),
          db.fetchScalar<int>('SELECT 1', []),
          db.fetchAll<int>('SELECT 1', [], (_) {
            called = true;
            return 1;
          }),
          db.transaction<Unit>((_) async {
            called = true;
            return const Ok(unit);
          }),
          db.close(),
          escaped.fetchScalar<int>('SELECT 1', []),
          escaped.transaction<Unit>((_) async => const Ok(unit)),
          escaped.close(),
        ])) {
          expect(result.unwrapErr().category, SqlxErrorCategory.transaction);
        }
        expect(called, isFalse);
      } finally {
        release.complete();
      }
      expect((await owner).unwrapErr(), same(aborted));
      expect(
        expectOk(await db.fetchScalar<int>('SELECT COUNT(*) FROM entries', [])),
        0,
      );
      expect((await escaped.execute('SELECT 1', [])).isErr, isTrue);
      expectOk(await escaped.close());
    },
  );

  test(
    'nested rollback preserves parent SQL and rejects parent/root misuse',
    () async {
      expectOk(
        await db.transaction<Unit>((tx) async {
          expect((await db.execute('SELECT 1', [])).isErr, isTrue);
          expect((await db.close()).isErr, isTrue);
          expectOk(await tx.execute('INSERT INTO entries VALUES (1)', []));
          final abort = SqlxError.transaction('child abort');
          final child = await tx.transaction<Unit>((nested) async {
            expect((await tx.execute('SELECT 1', [])).isErr, isTrue);
            expectOk(
              await nested.execute('INSERT INTO entries VALUES (2)', []),
            );
            return Err(abort);
          });
          expect(child.unwrapErr(), same(abort));
          expectOk(await tx.execute('INSERT INTO entries VALUES (3)', []));
          return const Ok(unit);
        }),
      );
      expect(
        expectOk(await db.fetchScalar<int>('SELECT SUM(id) FROM entries', [])),
        4,
      );
    },
  );

  test(
    'thrown callbacks roll back while unsendable values and errors stay local',
    () async {
      final thrown = await db.transaction<Unit>((tx) async {
        expectOk(await tx.execute('INSERT INTO entries VALUES (1)', []));
        throw StateError('callback');
      });
      expect(thrown.unwrapErr().operation, 'transaction callback');
      expect(
        expectOk(await db.fetchScalar<int>('SELECT COUNT(*) FROM entries', [])),
        0,
      );
      final port = ReceivePort();
      addTearDown(port.close);
      expect(
        expectOk(await db.transaction<ReceivePort>((_) async => Ok(port))),
        same(port),
      );
      final failure = SqlxError.transaction('unsendable cause', cause: port);
      expect(
        (await db.transaction<Unit>((_) async => Err(failure))).unwrapErr(),
        same(failure),
      );
      final badDiagnostic = SqlxError.transaction(
        'bad diagnostic',
        cause: _BadDiagnostic(),
      );
      expect(
        (await db.transaction<Unit>(
          (_) async => Err(badDiagnostic),
        )).unwrapErr(),
        same(badDiagnostic),
      );
    },
  );

  test('close accepts already-sent SQL once, then rejects new work', () async {
    final accepted = db.execute('INSERT INTO entries VALUES (1)', []);
    final close = db.close();
    expect(db.close(), same(close));
    expect(
      (await db.execute(
        'INSERT INTO entries VALUES (2)',
        [],
      )).unwrapErr().category,
      SqlxErrorCategory.connection,
    );
    expectOk(await accepted);
    expectOk(await close);
    expectOk(await db.close());
  });

  test(
    'stale callback zones cannot access a reusable root connection',
    () async {
      final release = Completer<void>();
      late Future<Result<int, SqlxError>> stale;
      expectOk(
        await db.transaction<Unit>((_) async {
          stale = release.future.then(
            (_) => db.fetchScalar<int>('SELECT 1', []),
          );
          return const Ok(unit);
        }),
      );
      release.complete();
      expect((await stale).unwrapErr().category, SqlxErrorCategory.transaction);
      expect(expectOk(await db.fetchScalar<int>('SELECT 2', [])), 2);
    },
  );
}

final class _BadDiagnostic {
  @override
  String toString() => throw StateError('diagnostic cannot be sent');
}
