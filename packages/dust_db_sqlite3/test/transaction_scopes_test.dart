import 'dart:async';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  late Sqlite3Driver db;
  setUp(() => db = Sqlite3Driver.open(':memory:'));
  tearDown(() async => expectOk(await db.close()));

  test(
    'root handles fail inside callback, including unchecked and close',
    () async {
      final results = await db.transaction<Unit>((tx) async {
        final unsafe = Sqlite3UnsafeSql(db);
        final calls = [
          db.execute('SELECT 1', []),
          db.fetchScalar<int>('SELECT 1', []),
          db.fetchOne<int>('SELECT 1 AS id', [], (row) => row.read<int>('id')),
          db.fetchOptional<int>(
            'SELECT 1 AS id',
            [],
            (row) => row.read<int>('id'),
          ),
          db.fetchAll<int>('SELECT 1 AS id', [], (row) => row.read<int>('id')),
          unsafe.fetch('SELECT 1', []),
          unsafe.fetchAs<int>(
            'SELECT 1 AS id',
            [],
            (row) => row.read<int>('id'),
          ),
          unsafe.execute('SELECT 1', []),
          db.transaction<Unit>((_) async => const Ok(unit)),
          db.close(),
        ];
        for (final result in await Future.wait(calls)) {
          final error = result.unwrapErr();
          expect(error.category, SqlxErrorCategory.transaction);
          expect(error.driver, Driver.sqlite3);
          expect(error.message, contains('current transaction executor'));
        }
        expect(expectOk(await tx.fetchScalar<int>('SELECT 1', [])), 1);
        return const Ok(unit);
      });
      expectOk(results);
      expect(expectOk(await db.fetchScalar<int>('SELECT 2', [])), 2);
    },
  );

  test(
    'active transaction handles cannot escape their callback zone',
    () async {
      final entered = Completer<void>();
      final release = Completer<void>();
      late Transaction escaped;
      final transaction = db.transaction<Unit>((tx) async {
        escaped = tx;
        entered.complete();
        await release.future;
        expect(expectOk(await tx.fetchScalar<int>('SELECT 1', [])), 1);
        return const Ok(unit);
      });
      await entered.future;
      try {
        for (final result in await Future.wait([
          escaped.fetchScalar<int>('SELECT 2', []),
          escaped.transaction<Unit>((_) async => const Ok(unit)),
          escaped.close(),
        ])) {
          expect(result.unwrapErr().category, SqlxErrorCategory.transaction);
          expect(
            result.unwrapErr().message,
            contains('only within its callback'),
          );
        }
      } finally {
        release.complete();
      }
      expectOk(await transaction);
      expect(
        (await escaped.fetchScalar<int>('SELECT 1', [])).unwrapErr().operation,
        'checkTransactionOpen',
      );
      expectOk(await escaped.close());
    },
  );

  test('parallel nested transactions and parent access are rejected', () async {
    expectOk(
      await db.transaction<Unit>((tx) async {
        final entered = Completer<void>();
        final release = Completer<void>();
        final nested = tx.transaction<Unit>((inner) async {
          entered.complete();
          expect(
            (await tx.fetchScalar<int>('SELECT 1', [])).unwrapErr().category,
            SqlxErrorCategory.transaction,
          );
          await release.future;
          expect(expectOk(await inner.fetchScalar<int>('SELECT 2', [])), 2);
          return const Ok(unit);
        });
        await entered.future;
        try {
          for (final result in await Future.wait([
            tx.transaction<Unit>((_) async => const Ok(unit)),
            tx.execute('SELECT 1', []),
            tx.close(),
          ])) {
            expect(
              result.unwrapErr().message,
              contains('active nested transaction'),
            );
          }
        } finally {
          release.complete();
        }
        expectOk(await nested);
        expect(expectOk(await tx.fetchScalar<int>('SELECT 3', [])), 3);
        return const Ok(unit);
      }),
    );
  });

  test(
    'parent waits for a live child even if callback forgot to await it',
    () async {
      final entered = Completer<void>();
      final release = Completer<void>();
      late Future<Result<Unit, SqlxError>> child;
      var settled = false;
      final root = db.transaction<Unit>((tx) async {
        child = tx.transaction<Unit>((inner) async {
          entered.complete();
          await release.future;
          return const Ok(unit);
        });
        return const Ok(unit);
      })..then((_) => settled = true);
      await entered.future;
      try {
        await Future<void>(() {});
        expect(settled, isFalse);
        expect(db.database.autocommit, isFalse);
      } finally {
        release.complete();
      }
      expectOk(await child);
      expectOk(await root);
      expect(db.database.autocommit, isTrue);
    },
  );

  test(
    'late async descendants cannot reuse a closed transaction context',
    () async {
      final release = Completer<void>();
      late Future<Result<int, SqlxError>> lateRoot;
      expectOk(
        await db.transaction<Unit>((tx) async {
          lateRoot = release.future.then(
            (_) => db.fetchScalar<int>('SELECT 1', []),
          );
          return const Ok(unit);
        }),
      );
      release.complete();
      expect((await lateRoot).unwrapErr().message, contains('scope is closed'));
      expect(expectOk(await db.fetchScalar<int>('SELECT 2', [])), 2);
    },
  );
}
