import 'dart:io';

import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

/// Opting in to a reviewed legacy baseline, not verifying historical SQL.
///
/// Run: `dart run example/legacy_migration_baseline.dart`.
Future<void> main() async {
  const sql = 'CREATE TABLE items (id INTEGER PRIMARY KEY);';
  const reviewedBundle = {'0001.sql': sql};
  final directory = Directory.systemTemp.createTempSync('dust-legacy-');
  final path = '${directory.path}/app.db';
  try {
    final legacy = Sqlite3Driver.open(path);
    try {
      // Simulate the history written by old Dust releases.
      legacy.database.execute('''
$sql
INSERT INTO items VALUES (7);
CREATE TABLE __dust_schema_migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL);
INSERT INTO __dust_schema_migrations VALUES ('0001.sql', '2025-01-01T00:00:00.000Z');
''');
    } finally {
      (await legacy.close()).unwrap();
    }
    final db = Sqlite3Driver.open(
      path,
      migrations: reviewedBundle,
      options: const SqliteConnectOptions(baselineLegacyMigrations: true),
    );
    try {
      final source = (await db.fetchScalar<String>(
        'SELECT checksum_source FROM __dust_schema_migrations',
        [],
      )).unwrap();
      print('history source: $source');
      print('historical SQL verified: false');
      print(
        'rows preserved: ${(await db.fetchScalar<int>('SELECT COUNT(*) FROM items', [])).unwrap()}',
      );
    } finally {
      (await db.close()).unwrap();
    }
  } finally {
    directory.deleteSync(recursive: true);
  }
}
