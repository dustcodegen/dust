import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

/// Repair data after a deferred migration COMMIT failure; add --background.
Future<void> main(List<String> args) async {
  final directory = Directory.systemTemp.createTempSync(
    'dust-migration-commit-',
  );
  const initial = {
    '0001.sql': '''
CREATE TABLE parents (id INTEGER PRIMARY KEY);
CREATE TABLE items (parent_id INTEGER REFERENCES parents(id) DEFERRABLE INITIALLY DEFERRED);
''',
  };
  const pending = {...initial, '0002.sql': 'INSERT INTO items VALUES (1);'};
  try {
    Future<Pool> open(Map<String, String> migrations) async {
      final path = '${directory.path}/app.db';
      const options = SqliteConnectOptions(foreignKeys: true);
      return args.contains('--background')
          ? await Sqlite3WorkerDriver.open(
              path,
              options: options,
              migrations: migrations,
            )
          : Sqlite3Driver.open(path, options: options, migrations: migrations);
    }

    final seed = await open(initial);
    (await seed.close()).unwrap();
    try {
      final unexpected = await open(pending);
      (await unexpected.close()).unwrap();
      throw StateError('expected a deferred foreign-key failure');
    } on SqlxError catch (error) {
      if (error.category != SqlxErrorCategory.migration ||
          error.operation != 'COMMIT') {
        rethrow;
      }
      print('migration failed at: ${error.operation}');
    }
    final repair = await open(initial);
    try {
      final count = (await queryScalar<int>(
        'SELECT COUNT(*) FROM items',
        [],
      ).fetchOne(repair)).unwrap();
      if (count != 0) throw StateError('failed migration left partial data');
      (await queryExecute(
        'INSERT INTO parents VALUES (1)',
        [],
      ).execute(repair)).unwrap();
    } finally {
      (await repair.close()).unwrap();
    }
    // Reapply the same bundle only after reconciling the missing parent data.
    final ready = await open(pending);
    try {
      final count = (await queryScalar<int>(
        'SELECT COUNT(*) FROM items',
        [],
      ).fetchOne(ready)).unwrap();
      print('dependency repaired: rows: $count');
    } finally {
      (await ready.close()).unwrap();
    }
  } finally {
    directory.deleteSync(recursive: true);
  }
}
