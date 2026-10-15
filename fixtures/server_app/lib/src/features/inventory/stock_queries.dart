import 'package:dust_dart/db.dart';

import 'stock_model.dart';

/// Requires exactly one matching row; zero or multiple rows are errors.
Future<Result<Stock, SqlxError>> onlyStock(Executor db, int minimum) =>
    queryAs<Stock>(
      r'SELECT item, on_hand FROM stock WHERE on_hand >= $1',
      [minimum],
    ).fetchOne(db);

/// First matching row in item order, or null when no row matches.
Future<Result<Stock?, SqlxError>> optionalStock(Executor db, int minimum) =>
    queryAs<Stock>(
      r'SELECT item, on_hand FROM stock WHERE on_hand >= $1 ORDER BY item',
      [minimum],
    ).fetchOptional(db);

/// Everything in stock, ordered just like the DAO query.
Future<Result<List<Stock>, SqlxError>> checkedStock(Executor db) =>
    queryAs<Stock>(
      'SELECT item, on_hand FROM stock ORDER BY item',
      [],
    ).fetchAll(db);

/// Binds a whole set as one value, including an empty set.
Future<Result<List<Stock>, SqlxError>> checkedStockForItems(
  Executor db,
  List<String> items,
) =>
    queryAs<Stock>(
      r'''
SELECT item, on_hand FROM stock
WHERE item IN (SELECT value FROM json_each($1))
ORDER BY item
''',
      [items],
    ).fetchAll(db);

/// Adds stock; repeated `$2` references still bind one supplied quantity.
Future<Result<ExecResult, SqlxError>> addCheckedStock(
  Executor db,
  String item,
  int quantity,
) =>
    queryExecute(
      r'''
INSERT INTO stock (item, on_hand) VALUES ($1, $2)
ON CONFLICT (item) DO UPDATE SET on_hand = on_hand + $2
''',
      [item, quantity],
    ).execute(db);

/// Total stock, including an empty inventory.
Future<Result<int, SqlxError>> checkedStockTotal(Executor db) =>
    queryScalar<int>(
      'SELECT COALESCE(SUM(on_hand), 0) FROM stock',
      [],
    ).fetchOne(db);
