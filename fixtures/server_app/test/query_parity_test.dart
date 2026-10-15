import 'dart:async';

import 'package:dust_dart/db.dart';
import 'package:server_app/server_app.dart';
import 'package:server_app/src/features/inventory/stock_queries.dart';
import 'package:test/test.dart';

import '../example/checked_queries.dart' as example;

void main() {
  for (final arguments in [
    <String>[],
    ['--background']
  ]) {
    test('checked example output $arguments', () async {
      final lines = <String>[];
      await runZoned(() => example.main(arguments),
          zoneSpecification: ZoneSpecification(
            print: (self, parent, zone, line) => lines.add(line),
          ));
      expect(lines, ['one: shirt:5', 'optional: null', 'rows: 1', 'total: 5']);
    });
  }
  for (final background in [false, true]) {
    for (final dao in [false, true]) {
      group('${background ? 'worker' : 'sync'} / ${dao ? 'DAO' : 'inline'}',
          () {
        late AppDatabase app;
        setUp(() async {
          app = background
              ? await AppDatabase.openBackground(':memory:')
              : AppDatabase.open(':memory:');
        });
        tearDown(() async => (await app.close()).unwrap());

        Future<Result<Stock, SqlxError>> one(Executor db, int minimum) =>
            dao ? InventoryRepo(db).onlyStock(minimum) : onlyStock(db, minimum);
        Future<Result<Stock?, SqlxError>> optional(Executor db, int minimum) =>
            dao
                ? InventoryRepo(db).optionalStock(minimum)
                : optionalStock(db, minimum);
        Future<Result<List<Stock>, SqlxError>> all(Executor db) =>
            dao ? InventoryRepo(db).allStock() : checkedStock(db);
        Future<Result<int, SqlxError>> total(Executor db) =>
            dao ? InventoryRepo(db).totalStock() : checkedStockTotal(db);
        Future<Result<ExecResult, SqlxError>> save(
          Executor db,
          String item,
          int quantity,
        ) =>
            dao
                ? InventoryRepo(db).addStock(item, quantity)
                : addCheckedStock(db, item, quantity);
        Future<Result<List<Stock>, SqlxError>> items(
          Executor db,
          List<String> names,
        ) =>
            dao
                ? InventoryRepo(db).stockForItems(names)
                : checkedStockForItems(db, names);

        test('five terminals retain values, counts and cardinality', () async {
          final db = app.connection;
          expect((await all(db)).unwrap(), <Stock>[]);
          expect((await optional(db, 0)).unwrap(), isNull);
          expect((await total(db)).unwrap(), 0);
          final missing = (await one(db, 0)).unwrapErr();
          expect(missing, isA<SqlxCardinalityError>());
          expect((missing as SqlxCardinalityError).actual, 0);

          final inserted = (await save(db, 'shirt', 5)).unwrap();
          expect(inserted.rowsAffected, 1);
          expect(inserted.lastInsertId, 1);
          expect((await save(db, 'shirt', 2)).unwrap().rowsAffected, 1);
          const shirt = Stock(item: 'shirt', onHand: 7);
          expect((await one(db, 0)).unwrap(), shirt);
          expect((await optional(db, 0)).unwrap(), shirt);
          expect((await all(db)).unwrap(), [shirt]);

          (await save(db, 'socks', 3)).unwrap();
          expect((await total(db)).unwrap(), 10);
          expect((await all(db)).unwrap(), [
            shirt,
            const Stock(item: 'socks', onHand: 3),
          ]);
          expect((await optional(db, 0)).unwrap(), shirt);
          final error = (await one(db, 0)).unwrapErr();
          expect(error, isA<SqlxCardinalityError>());
          expect((error as SqlxCardinalityError).actual, 2);
          expect(error.driver, Driver.sqlite3);
          expect(error.operation,
              r'SELECT item, on_hand FROM stock WHERE on_hand >= $1');
        });

        test('one list bind handles empty and populated sets', () async {
          final db = app.connection;
          (await save(db, 'shirt', 5)).unwrap();
          (await save(db, 'socks', 3)).unwrap();
          expect((await items(db, [])).unwrap(), <Stock>[]);
          expect((await items(db, ['socks', 'missing'])).unwrap(), [
            const Stock(item: 'socks', onHand: 3),
          ]);
        });

        test('constraint errors preserve data and permit reuse', () async {
          final db = app.connection;
          (await save(db, 'shirt', 5)).unwrap();
          final error = (await save(db, 'shirt', -6)).unwrapErr();
          expect(error.category, SqlxErrorCategory.query);
          expect(error.kind, SqlxErrorKind.checkViolation);
          expect(error.driver, Driver.sqlite3);
          expect((await total(db)).unwrap(), 5);
          (await save(db, 'shirt', 1)).unwrap();
          expect((await total(db)).unwrap(), 6);
        });

        test('commit, nested rollback and escaped executors agree', () async {
          late Transaction escaped;
          (await app.transaction<Unit>((tx) async {
            escaped = tx;
            (await save(tx, 'shirt', 5)).unwrap();
            final rejected = await tx.transaction<Unit>((child) async {
              (await save(child, 'socks', 3)).unwrap();
              return Err(SqlxError.transaction('cancel nested'));
            });
            expect(rejected.unwrapErr().message, 'cancel nested');
            expect((await total(tx)).unwrap(), 5);
            expect((await total(app.connection)).unwrapErr().category,
                SqlxErrorCategory.transaction);
            return const Ok(unit);
          }))
              .unwrap();
          expect((await total(app.connection)).unwrap(), 5);
          expect((await total(escaped)).unwrapErr().category,
              SqlxErrorCategory.transaction);

          final rejected = await app.transaction<Unit>((tx) async {
            (await save(tx, 'socks', 3)).unwrap();
            return Err(SqlxError.transaction('cancel outer'));
          });
          expect(rejected.unwrapErr().message, 'cancel outer');
          expect((await all(app.connection)).unwrap(), [
            const Stock(item: 'shirt', onHand: 5),
          ]);
        });

        test('all terminals reject a closed connection', () async {
          (await app.close()).unwrap();
          final db = app.connection;
          final results = [
            await one(db, 0),
            await optional(db, 0),
            await all(db),
            await total(db),
            await save(db, 'shirt', 1),
          ];
          for (final result in results) {
            expect(result.unwrapErr().category, SqlxErrorCategory.connection);
          }
        });
      });
    }
  }
}
