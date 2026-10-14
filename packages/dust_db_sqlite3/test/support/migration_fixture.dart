import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

import 'expect_ok.dart';

const createItems =
    'CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL);';
const upgradeItems =
    'ALTER TABLE items ADD COLUMN active INTEGER NOT NULL DEFAULT 1;';
const createChecksum =
    '41a4749ebe427468c958195bcab1c902b9d28dcea9f2a174cebee1d82e1ac65f';
const upgradeChecksum =
    '7e500dada2af7a66826253dfc77792102b645c0c8acdac90944ff8a13e07d82e';
const Map<String, String> firstMigration = {'0001.sql': createItems};
const Map<String, String> upgradedMigrations = {
  '0001.sql': createItems,
  '0002.sql': upgradeItems,
};
const legacyTimestamp = '2025-01-01T00:00:00.000Z';

/// Native inspection happens only between managed opens.
final class MigrationFixture {
  MigrationFixture() {
    directory = Directory.systemTemp.createTempSync('dust-integrity-');
    path = '${directory.path}/app.db';
  }

  late final Directory directory;
  late final String path;

  Sqlite3Driver open({Map<String, String>? migrations, bool baseline = false}) {
    final db = Sqlite3Driver.open(
      path,
      migrations: migrations,
      options: SqliteConnectOptions(baselineLegacyMigrations: baseline),
    );
    addTearDown(() async => expectOk(await db.close()));
    return db;
  }

  void dispose() => directory.deleteSync(recursive: true);

  void prepare(String sql) {
    final native = sqlite.sqlite3.open(path);
    try {
      native.execute(sql);
    } finally {
      native.close();
    }
  }

  List<List<Object?>> read(String sql) {
    final native = sqlite.sqlite3.open(path, mode: sqlite.OpenMode.readOnly);
    try {
      return [for (final row in native.select(sql)) row.values.toList()];
    } finally {
      native.close();
    }
  }

  void seedLegacy() => prepare('''
$createItems
INSERT INTO items VALUES (1, 'Ada 🍜');
CREATE TABLE __dust_schema_migrations (
  name TEXT PRIMARY KEY, applied_at TEXT NOT NULL
);
INSERT INTO __dust_schema_migrations VALUES ('0001.sql', '$legacyTimestamp');
''');

  void expectError(
    Map<String, String> migrations,
    String name,
    String message, {
    bool baseline = false,
  }) {
    expect(
      () => open(migrations: migrations, baseline: baseline),
      throwsA(
        isA<SqlxError>()
            .having((e) => e.category, 'category', SqlxErrorCategory.migration)
            .having((e) => e.driver, 'driver', Driver.sqlite3)
            .having((e) => e.operation, 'operation', name)
            .having((e) => e.message, 'message', message),
      ),
    );
  }
}
