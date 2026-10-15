import 'dart:async';

import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import '../example/background_database.dart' as example;
import 'testing.dart';

void main() {
  test('generated worker facade runs the real HTTP checkout flow', () async {
    final app = await TestApp.start(background: true);
    addTearDown(app.stop);
    expect(app.database.connection, isA<Sqlite3WorkerDriver>());
    expect(app.database.connection, isNot(isA<Sqlite3Executor>()));
    (await app.database.migrate()).unwrap();
    await app.createAccount('worker@example.test', 'correct horse battery');
    final token =
        await app.signIn('worker@example.test', 'correct horse battery');
    (await app.inventory.addStock('shirt', 5)).unwrap();
    final response = await (app.client.post('/inventory/checkout')
          ..bearer(token)
          ..json({'item': 'shirt', 'quantity': 2}))
        .send();
    response.assertCreated();
    expect((response.json as Map<String, dynamic>)['quantity'], 2);
    final stock = (await app.inventory.stockFor('shirt')).unwrap()!;
    expect(stock.onHand, 3);
    final count = (await app.database.unsafe.fetch(
      'SELECT COUNT(*) AS count FROM orders',
      [],
    ))
        .unwrap();
    expect(count.single.read<int>('count'), 1);
  });

  test('background database example produces the documented output', () async {
    final lines = <String>[];
    await runZoned(example.main,
        zoneSpecification: ZoneSpecification(
          print: (self, parent, zone, line) => lines.add(line),
        ));
    expect(lines, ['stock: 5', 'total: 5', 'table: stock']);
  });
}
