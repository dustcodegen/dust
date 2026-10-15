import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show initializeSqlite3Connection;
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

void main() {
  late Directory directory;
  late String path;

  setUp(() {
    directory = Directory.systemTemp.createTempSync('dust_migration_failure_');
    path = '${directory.path}/app.db';
  });
  tearDown(() => directory.deleteSync(recursive: true));

  void prepare(String sql) {
    final native = sqlite.sqlite3.open(path);
    try {
      native.execute(sql);
    } finally {
      native.close();
    }
  }

  SqlxError openingError(
    String sql, {
    bool foreignKeys = false,
    Map<String, Object> pragmas = const {},
  }) {
    try {
      final db = Sqlite3Driver.open(
        path,
        options: SqliteConnectOptions(
          foreignKeys: foreignKeys,
          pragmas: pragmas,
        ),
        migrations: {'0001.sql': sql},
      );
      db.database.close();
      fail('expected startup failure');
    } on SqlxError catch (error) {
      expect(error.category, SqlxErrorCategory.migration);
      expect(error.driver, Driver.sqlite3);
      expect(error.cause, isA<sqlite.SqliteException>());
      return error;
    }
  }

  void expectNoItems() {
    final native = sqlite.sqlite3.open(path);
    try {
      expect(
        native.select(
          "SELECT name FROM sqlite_master WHERE name = 'items'",
        ),
        isEmpty,
      );
      expect(native.autocommit, isTrue);
    } finally {
      native.close();
    }
  }

  test(
    'migration BEGIN failure reports the operation and closes the owned handle',
    () {
      final native = sqlite.sqlite3.openInMemory();
      addTearDown(native.close);
      native.execute('BEGIN');
      expect(
        () => initializeSqlite3Connection(
          native,
          const SqliteConnectOptions.memory(),
          {'0001.sql': 'SELECT 1;'},
        ),
        throwsA(
          isA<SqlxError>()
              .having(
                (e) => e.category,
                'category',
                SqlxErrorCategory.migration,
              )
              .having((e) => e.operation, 'operation', 'BEGIN IMMEDIATE')
              .having((e) => e.cause, 'cause', isA<sqlite.SqliteException>()),
        ),
      );
      expect(() => native.select('SELECT 1'), throwsA(isA<StateError>()));
    },
  );

  test('migration table setup failure retains its native cause', () {
    prepare('CREATE VIEW __dust_schema_migrations AS SELECT 1 AS name;');
    final error = openingError(
      'CREATE TABLE items (id INTEGER);',
    );
    expect(error.operation, '__dust_schema_migrations');
    expect(error.message, 'SQLite migration table setup failed.');
    expectNoItems();
  });

  test('malformed migration history is rejected before applying SQL', () {
    prepare('CREATE TABLE __dust_schema_migrations (wrong TEXT);');
    final error = openingError('CREATE TABLE items (id INTEGER);');
    expect(error.operation, '__dust_schema_migrations');
    expect(error.message, 'SQLite migration history read failed.');
    expectNoItems();
  });

  test('history prepare failure rolls back the migration schema', () {
    final error = openingError('''
CREATE TABLE items (id INTEGER);
DROP TABLE __dust_schema_migrations;
''');
    expect(error.operation, '0001.sql');
    expect(error.message, 'SQLite migration history prepare failed.');
    expectNoItems();
  });

  test(
    'history write failure rolls back SQL and does not record a migration',
    () {
      prepare('''
CREATE TABLE __dust_schema_migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL);
CREATE TRIGGER reject_history BEFORE INSERT ON __dust_schema_migrations
BEGIN SELECT RAISE(ABORT, 'history denied'); END;
''');
      final error = openingError('CREATE TABLE items (id INTEGER);');
      expect(error.operation, '0001.sql');
      expect(error.message, 'SQLite migration history write failed.');
      expectNoItems();
      final native = sqlite.sqlite3.open(path);
      try {
        expect(
          native.select('SELECT * FROM __dust_schema_migrations'),
          isEmpty,
        );
      } finally {
        native.close();
      }
    },
  );

  test(
    'deferred constraint at migration COMMIT rolls back all startup SQL',
    () {
      final error = openingError('''
CREATE TABLE parents (id INTEGER PRIMARY KEY);
CREATE TABLE items (parent_id INTEGER REFERENCES parents(id) DEFERRABLE INITIALLY DEFERRED);
INSERT INTO items VALUES (1);
''', foreignKeys: true);
      expect(error.message, 'SQLite migration failed.');
      expect(error.operation, 'COMMIT');
      expect((error.cause! as sqlite.SqliteException).extendedResultCode, 787);
      expectNoItems();
    },
  );

  test('an already ended migration retains its original SQL failure', () {
    final error = openingError('''
CREATE TABLE items (id INTEGER);
ROLLBACK;
THIS IS NOT SQL;
''');
    expect(error.operation, '0001.sql');
    expect(error.message, 'SQLite migration `0001.sql` failed.');
    expectNoItems();
  });
}
