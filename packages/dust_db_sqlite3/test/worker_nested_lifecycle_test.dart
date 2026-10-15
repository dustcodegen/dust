import 'dart:async';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  for (final outcome in ['commit', 'returned error', 'throw']) {
    test(
      'worker root $outcome joins children and expires the callback first',
      () async {
        final db = await Sqlite3WorkerDriver.open(
          ':memory:',
          migrations: const {'0001.sql': 'CREATE TABLE entries (id INTEGER);'},
        );
        addTearDown(db.close);
        final entered = Completer<void>();
        final callbackReturned = Completer<void>();
        final release = Completer<void>();
        late Transaction parent;
        late Future<Result<Unit, SqlxError>> child;
        late Future<Result<Unit, SqlxError>> grandchild;
        var settled = false;
        final owner = db.transaction<Unit>((tx) async {
          parent = tx;
          child = tx.transaction<Unit>((inner) async {
            grandchild = inner.transaction<Unit>((leaf) async {
              entered.complete();
              await release.future;
              return (await leaf.execute(
                'INSERT INTO entries VALUES (1)',
                [],
              )).map((_) => unit);
            });
            return const Ok(unit);
          });
          callbackReturned.complete();
          if (outcome == 'throw') throw StateError('abort');
          if (outcome == 'returned error') {
            return Err(SqlxError.transaction('abort'));
          }
          return const Ok(unit);
        })..then((_) => settled = true);
        await entered.future;
        await callbackReturned.future;
        try {
          await Future<void>(() {});
          expect(settled, isFalse);
          expect(
            (await parent.execute('SELECT 1', [])).unwrapErr().message,
            'SQLite transaction is closed.',
          );
          expect(
            (await db.close()).unwrapErr().category,
            SqlxErrorCategory.transaction,
          );
        } finally {
          release.complete();
        }
        expectOk(await grandchild);
        expectOk(await child);
        expect((await owner).isOk, outcome == 'commit');
        expect(
          expectOk(
            await db.fetchScalar<int>('SELECT COUNT(*) FROM entries', []),
          ),
          outcome == 'commit' ? 1 : 0,
        );
      },
    );
  }

  test(
    'active nested work rejects parent access, sibling transactions and close',
    () async {
      final db = await Sqlite3WorkerDriver.open(':memory:');
      addTearDown(db.close);
      expectOk(
        await db.transaction<Unit>((tx) async {
          final entered = Completer<void>();
          final release = Completer<void>();
          final child = tx.transaction<Unit>((inner) async {
            entered.complete();
            await release.future;
            return const Ok(unit);
          });
          await entered.future;
          try {
            for (final result in await Future.wait([
              tx.execute('SELECT 1', []),
              tx.transaction<Unit>((_) async => const Ok(unit)),
              tx.close(),
            ])) {
              expect(
                result.unwrapErr().category,
                SqlxErrorCategory.transaction,
              );
            }
          } finally {
            release.complete();
          }
          expectOk(await child);
          expectOk(await tx.execute('SELECT 1', []));
          return const Ok(unit);
        }),
      );
    },
  );
}
