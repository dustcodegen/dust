import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

/// Safe page-limit demonstration, not real filesystem exhaustion.
Future<void> main(List<String> args) async {
  final directory = Directory.systemTemp.createTempSync('dust-full-example-');
  try {
    final path = '${directory.path}/app.db';
    const migrations = {
      '0001.sql': 'CREATE TABLE jobs (id INTEGER PRIMARY KEY, payload BLOB);',
    };
    final db = args.contains('--background')
        ? await Sqlite3WorkerDriver.open(path, migrations: migrations)
        : Sqlite3Driver.open(path, migrations: migrations);
    try {
      final capacity = (await queryScalar<int>(
        'PRAGMA max_page_count',
        [],
      ).fetchOne(db)).unwrap();
      final pages = (await queryScalar<int>(
        'PRAGMA page_count',
        [],
      ).fetchOne(db)).unwrap();
      (await queryScalar<int>(
        'PRAGMA max_page_count = ${pages + 2}',
        [],
      ).fetchOne(db)).unwrap();
      Future<Result<Unit, SqlxError>> saveJob() =>
          db.transaction<Unit>((tx) async {
            (await queryExecute(
              'INSERT INTO jobs VALUES (1, NULL)',
              [],
            ).execute(tx)).unwrap();
            return (await queryExecute(
              'UPDATE jobs SET payload = zeroblob(262144) WHERE id = 1',
              [],
            ).execute(tx)).map((_) => unit);
          });
      final failed = await saveJob();
      if (failed.isOk) throw StateError('expected the test page limit to fail');
      final count = (await queryScalar<int>(
        'SELECT COUNT(*) FROM jobs',
        [],
      ).fetchOne(db)).unwrap();
      if (count != 0) throw StateError('failed job must not be persisted');
      print('page limit: write failed, rows: $count');

      // Only this owned demonstration DB has an artificial cap to restore.
      (await queryScalar<int>(
        'PRAGMA max_page_count = $capacity',
        [],
      ).fetchOne(db)).unwrap();
      (await saveJob()).unwrap(); // Deliberate retry, never automatic replay.
      final restored = (await queryScalar<int>(
        'SELECT COUNT(*) FROM jobs',
        [],
      ).fetchOne(db)).unwrap();
      print('capacity restored: rows: $restored');
    } finally {
      (await db.close()).unwrap();
    }
  } finally {
    directory.deleteSync(recursive: true);
  }
}
