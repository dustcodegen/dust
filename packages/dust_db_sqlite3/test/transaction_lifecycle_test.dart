import 'dart:async';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  for (final outcome in ['commit', 'returned error', 'throw']) {
    test('root $outcome waits for unawaited child and grandchild', () async {
      final db = Sqlite3Driver.open(
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
        if (outcome == 'throw') throw StateError('abort parent');
        if (outcome == 'returned error') {
          return Err(SqlxError.transaction('abort parent'));
        }
        return const Ok(unit);
      })..then((_) => settled = true);
      await entered.future;
      await callbackReturned.future;
      try {
        // Advance the event loop, without a timing-dependent sleep.
        await Future<void>(() {});
        expect(settled, isFalse);
        expect(db.database.autocommit, isFalse);
        expect(
          (await parent.fetchScalar<int>('SELECT 1', [])).unwrapErr().operation,
          'checkTransactionOpen',
        );
        expect(
          (await db.execute(
            'INSERT INTO entries VALUES (2)',
            [],
          )).unwrapErr().operation,
          'checkTransactionOwner',
        );
        expect((await db.close()).isErr, isTrue);
      } finally {
        release.complete();
      }
      expectOk(await grandchild);
      expectOk(await child);
      expect((await owner).isOk, outcome == 'commit');
      expect(db.database.autocommit, isTrue);
      expect(
        expectOk(await db.fetchScalar<int>('SELECT COUNT(*) FROM entries', [])),
        outcome == 'commit' ? 1 : 0,
      );
      expectOk(await db.transaction<Unit>((_) async => const Ok(unit)));
    });
  }
}
