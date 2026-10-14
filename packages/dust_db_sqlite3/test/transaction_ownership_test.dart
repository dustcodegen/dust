import 'dart:async';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  for (final outcome in ['commit', 'returned error', 'throw']) {
    test('outside work is rejected across an await before $outcome', () async {
      final db = Sqlite3Driver.open(
        ':memory:',
        migrations: const {'0001.sql': 'CREATE TABLE entries (id INTEGER);'},
      );
      addTearDown(db.close);
      final entered = Completer<void>();
      final release = Completer<void>();
      final owner = db.transaction<Unit>((tx) async {
        expectOk(await tx.execute('INSERT INTO entries VALUES (1)', []));
        entered.complete();
        await release.future;
        expect(
          expectOk(
            await tx.fetchScalar<int>('SELECT COUNT(*) FROM entries', []),
          ),
          1,
        );
        if (outcome == 'throw') throw StateError('abort');
        if (outcome == 'returned error') {
          return Err(SqlxError.transaction('abort'));
        }
        return const Ok(unit);
      });
      await entered.future;
      var callbackCalled = false;
      var mapperCalled = false;
      int mapper(Row row) {
        mapperCalled = true;
        return row.read<int>('id');
      }

      final unsafe = Sqlite3UnsafeSql(db);
      try {
        final calls = [
          db.execute('INSERT INTO entries VALUES (2)', []),
          db.fetchScalar<int>('SELECT COUNT(*) FROM entries', []),
          db.fetchOne<int>('SELECT id FROM entries', [], mapper),
          db.fetchOptional<int>('SELECT id FROM entries', [], mapper),
          db.fetchAll<int>('SELECT id FROM entries', [], mapper),
          unsafe.fetch('SELECT id FROM entries', []),
          unsafe.fetchAs<int>('SELECT id FROM entries', [], mapper),
          unsafe.execute('INSERT INTO entries VALUES (3)', []),
          db.transaction<Unit>((_) async {
            callbackCalled = true;
            return const Ok(unit);
          }),
          db.close(),
        ];
        for (final result in await Future.wait(calls)) {
          final error = result.unwrapErr();
          expect(error.category, SqlxErrorCategory.transaction);
          expect(error.driver, Driver.sqlite3);
          expect(error.operation, 'checkTransactionOwner');
        }
        expect(callbackCalled, isFalse);
        expect(mapperCalled, isFalse);
        expect(db.database.autocommit, isFalse);
      } finally {
        release.complete();
      }
      final result = await owner;
      expect(result.isOk, outcome == 'commit');
      expect(db.database.autocommit, isTrue);
      expect(
        expectOk(await db.fetchScalar<int>('SELECT COUNT(*) FROM entries', [])),
        outcome == 'commit' ? 1 : 0,
      );
      expectOk(
        await db.transaction<Unit>(
          (tx) async => (await tx.execute(
            'INSERT INTO entries VALUES (4)',
            [],
          )).map((_) => unit),
        ),
      );
      expectOk(await db.close());
      expectOk(await db.close());
    });
  }

  test(
    'ownership is per connection, including inside another callback',
    () async {
      final first = Sqlite3Driver.open(':memory:');
      final second = Sqlite3Driver.open(':memory:');
      addTearDown(first.close);
      addTearDown(second.close);
      final entered = Completer<void>();
      final release = Completer<void>();
      final owner = first.transaction<Unit>((tx) async {
        expect(expectOk(await second.fetchScalar<int>('SELECT 1', [])), 1);
        expectOk(
          await second.transaction<Unit>(
            (inner) async =>
                (await inner.fetchScalar<int>('SELECT 2', [])).map((_) => unit),
          ),
        );
        entered.complete();
        await release.future;
        return const Ok(unit);
      });
      await entered.future;
      try {
        expect(expectOk(await second.fetchScalar<int>('SELECT 3', [])), 3);
        expectOk(await second.close());
      } finally {
        release.complete();
      }
      expectOk(await owner);
    },
  );
}
