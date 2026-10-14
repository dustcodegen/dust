import 'dart:async';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

/// A single connection belongs to its current sqlx-style transaction.
///
/// Run with `dart run example/transaction_ownership.dart`.
Future<void> main() async {
  final db = Sqlite3Driver.open(
    ':memory:',
    migrations: const {'0001.sql': 'CREATE TABLE entries (id INTEGER);'},
  );
  final entered = Completer<void>();
  final release = Completer<void>();
  final owner = db.transaction<Unit>((tx) async {
    entered.complete();
    await release.future;
    return (await tx.execute(
      'INSERT INTO entries VALUES (1)',
      [],
    )).map((_) => unit);
  });
  try {
    await entered.future;
    final outside = await db.execute('INSERT INTO entries VALUES (2)', []);
    print('outside write rejected: ${outside.isErr}');
    final close = await db.close();
    print('early close rejected: ${close.isErr}');
    release.complete();
    print('owner committed: ${(await owner).isOk}');
    final count = await db.fetchScalar<int>('SELECT COUNT(*) FROM entries', []);
    print('rows: ${count.unwrapOrElse((error) => throw error)}');
  } finally {
    if (!release.isCompleted) release.complete();
    await owner;
    await db.close();
  }
}
