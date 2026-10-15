import 'package:dust_dart/db.dart';

import '../../features/inventory/stock_model.dart';

/// Direct, checked query calls with `db` or `tx`; no DAO or executor extension.
///
/// Kept in `lib/` so DB mode validates the example's SQL, not just its output.
Future<List<String>> runCheckedQueries(Executor db) async {
  (await db.transaction((tx) {
    return queryExecute(
      r'INSERT INTO stock (item, on_hand) VALUES ($1, $2)',
      ['shirt', 5],
    ).execute(tx);
  }))
      .unwrap();

  final one = (await queryAs<Stock>(
    r'SELECT item, on_hand FROM stock WHERE on_hand >= $1',
    [1],
  ).fetchOne(db))
      .unwrap();
  final optional = (await queryAs<Stock>(
    r'SELECT item, on_hand FROM stock WHERE on_hand >= $1 ORDER BY item',
    [10],
  ).fetchOptional(db))
      .unwrap();
  final all = (await queryAs<Stock>(
    'SELECT item, on_hand FROM stock ORDER BY item',
    [],
  ).fetchAll(db))
      .unwrap();
  final total = (await queryScalar<int>(
    'SELECT COALESCE(SUM(on_hand), 0) FROM stock',
    [],
  ).fetchOne(db))
      .unwrap();
  return [
    'one: ${one.item}:${one.onHand}',
    'optional: $optional',
    'rows: ${all.length}',
    'total: $total',
  ];
}
