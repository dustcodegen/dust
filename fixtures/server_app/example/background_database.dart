import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:server_app/server_app.dart';

/// Use a generated worker facade with checked DAOs and non-DAO SQLx helpers.
///
/// `dart run example/background_database.dart`
Future<void> main() async {
  final directory =
      await Directory.systemTemp.createTemp('dust-server-worker-');
  try {
    final db = await AppDatabase.openBackground(
      '${directory.path}/app.db',
      options: appOptions,
    );
    try {
      final saved = await db.transaction((tx) async {
        return InventoryRepo(tx).addStock('shirt', 5);
      });
      saved.unwrap();
      final stock =
          (await InventoryRepo(db.connection).stockFor('shirt')).unwrap();
      print('stock: ${stock!.onHand}');
      final total = (await db.connection.totalStock()).unwrap();
      print('total: $total');
      final tables = (await db.unsafe.fetchAs(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'stock'",
        [],
        (row) => row.read<String>('name'),
      ))
          .unwrap();
      print('table: ${tables.single}');
    } finally {
      (await db.close()).unwrap();
    }
  } finally {
    await directory.delete(recursive: true);
  }
}
