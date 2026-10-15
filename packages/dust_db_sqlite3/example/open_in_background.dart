import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

/// Await native opening, migrations and SQL without blocking the caller.
///
/// Row mappers still run on the caller. Use bounded queries for large tables.
/// `dart run example/open_in_background.dart`
Future<void> main() async {
  final directory = await Directory.systemTemp.createTemp('dust-worker-');
  try {
    final db = await Sqlite3WorkerDriver.open(
      '${directory.path}/app.db',
      migrations: const {
        '0001.sql': 'CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT);',
      },
      options: const SqliteConnectOptions(
        foreignKeys: true,
        busyTimeout: Duration(seconds: 5),
      ),
    );
    try {
      final saved = await db.transaction<Unit>((tx) async {
        return (await tx.execute(
          r'INSERT INTO notes (body) VALUES ($1)',
          ['Opened in the worker'],
        )).map((_) => unit);
      });
      saved.unwrap();
      final notes = (await db.fetchAll(
        'SELECT body FROM notes ORDER BY id LIMIT 20',
        [],
        (row) => row.read<String>('body'),
      )).unwrap();
      print('notes: $notes');
    } finally {
      (await db.close()).unwrap();
    }
  } finally {
    await directory.delete(recursive: true);
  }
}
