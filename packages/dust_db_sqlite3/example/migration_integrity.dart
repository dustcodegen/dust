import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

/// Detecting changed or missing migrations without touching application data.
///
/// This runtime map complements Dust/sqlx build-time SQL validation.
/// Run: `dart run example/migration_integrity.dart`.
Future<void> main() async {
  const sql = 'CREATE TABLE items (id INTEGER PRIMARY KEY);';
  const bundle = {'0001.sql': sql};
  final directory = Directory.systemTemp.createTempSync('dust-integrity-');
  final path = '${directory.path}/app.db';
  try {
    final first = Sqlite3Driver.open(path, migrations: bundle);
    try {
      (await first.execute('INSERT INTO items VALUES (7)', [])).unwrap();
    } finally {
      (await first.close()).unwrap();
    }
    print(
      'changed SQL rejected: ${await _rejected(path, {'0001.sql': '$sql '})}',
    );
    print('missing SQL rejected: ${await _rejected(path, {})}');
    final reopened = Sqlite3Driver.open(path, migrations: bundle);
    try {
      print(
        'rows preserved: ${(await reopened.fetchScalar<int>('SELECT COUNT(*) FROM items', [])).unwrap()}',
      );
    } finally {
      (await reopened.close()).unwrap();
    }
  } finally {
    directory.deleteSync(recursive: true);
  }
}

Future<bool> _rejected(String path, Map<String, String> migrations) async {
  try {
    final db = Sqlite3Driver.open(path, migrations: migrations);
    (await db.close()).unwrap();
    return false;
  } on SqlxError catch (error) {
    if (error.category != SqlxErrorCategory.migration) rethrow;
    return true;
  }
}
