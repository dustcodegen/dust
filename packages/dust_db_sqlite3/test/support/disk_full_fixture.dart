import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

import 'expect_ok.dart';

const fullMigrations = {
  '0001.sql': '''
CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL, payload BLOB);
''',
};
const fullInsert = "INSERT INTO items VALUES (3, 'large', zeroblob(262144))";
const fullReturning = '$fullInsert RETURNING id';

final class DiskFullFixture {
  DiskFullFixture(this.journal, {required this.background});

  final String journal;
  final bool background;
  final Directory directory = Directory.systemTemp.createTempSync('dust-full-');
  late final String path = '${directory.path}/app.db';
  late Pool db;
  late int originalLimit;
  late int limit;
  late List<List<String>> history;

  Future<Pool> open({Map<String, String>? migrations, int? pageLimit}) async {
    final options = SqliteConnectOptions(
      journalMode: SqliteJournalMode.values.byName(journal),
      synchronous: SqliteSynchronousMode.full,
      pragmas: {'max_page_count': ?pageLimit},
    );
    return background
        ? await Sqlite3WorkerDriver.open(
            path,
            options: options,
            migrations: migrations,
          )
        : Sqlite3Driver.open(path, options: options, migrations: migrations);
  }

  Future<void> seed() async {
    addTearDown(() => directory.deleteSync(recursive: true));
    db = await open(migrations: fullMigrations);
    addTearDown(() async => expectOk(await db.close()));
    expectOk(
      await db.execute("INSERT INTO items VALUES (1, 'baseline 🍜', NULL)", []),
    );
    history = await readHistory(db);
    originalLimit = expectOk(
      await db.fetchScalar<int>('PRAGMA max_page_count', []),
    );
    limit = expectOk(await db.fetchScalar<int>('PRAGMA page_count', [])) + 2;
    expect(
      expectOk(await db.fetchScalar<int>('PRAGMA max_page_count = $limit', [])),
      limit,
    );
  }

  Future<void> expectBaseline(Pool connection) async {
    expect(
      expectOk(
        await connection.fetchAll(
          'SELECT id, name, length(payload) AS size FROM items ORDER BY id',
          [],
          (row) => [
            row.read<int>('id'),
            row.read<String>('name'),
            row.readNullable<int>('size'),
          ],
        ),
      ),
      [
        [1, 'baseline 🍜', null],
      ],
    );
    expect(await readHistory(connection), history);
    expect(
      expectOk(
        await connection.fetchScalar<String>('PRAGMA integrity_check', []),
      ),
      'ok',
    );
  }

  Future<void> retry() async {
    expect(
      expectOk(
        await db.fetchScalar<int>('PRAGMA max_page_count = $originalLimit', []),
      ),
      originalLimit,
    );
    expectOk(
      await db.transaction<Unit>(
        (tx) async =>
            (await queryExecute(fullInsert, []).execute(tx)).map((_) => unit),
      ),
    );
    expect(
      expectOk(
        await db.fetchScalar<int>(
          'SELECT length(payload) FROM items WHERE id = 3',
          [],
        ),
      ),
      262144,
    );
    expectOk(
      await db.transaction<Unit>(
        (tx) async => (await tx.execute(
          "INSERT INTO items VALUES (4, 'reused', NULL)",
          [],
        )).map((_) => unit),
      ),
    );
  }
}

Future<List<List<String>>> readHistory(Executor db) async => expectOk(
  await db.fetchAll(
    'SELECT name, applied_at, checksum, checksum_source FROM __dust_schema_migrations ORDER BY name',
    [],
    (row) => [
      for (final name in ['name', 'applied_at', 'checksum', 'checksum_source'])
        row.read<String>(name),
    ],
  ),
);

void expectFull(
  SqlxError error,
  String operation, {
  required bool background,
  SqlxErrorCategory category = SqlxErrorCategory.query,
}) {
  expect(error.driver, Driver.sqlite3);
  expect(error.category, category);
  expect(error.operation, operation);
  if (background) {
    expect(error.cause, contains('SqliteException(13)'));
  } else {
    expect(
      error.cause,
      isA<sqlite.SqliteException>().having(
        (e) => e.resultCode,
        'SQLITE_FULL',
        13,
      ),
    );
  }
}
