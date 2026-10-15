import 'dart:convert';

import 'package:dust_dart/db.dart';

const initialMigrations = {
  '0001.sql': '''
CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL, payload BLOB);
INSERT INTO items VALUES (1, 'baseline 🍜', CAST(replace(hex(zeroblob(32768)), '00', 'ab') AS BLOB));
''',
};
const Map<String, String> pendingMigrations = {
  ...initialMigrations,
  '0002.sql': '''
CREATE TABLE extra (id INTEGER PRIMARY KEY, payload BLOB);
INSERT INTO extra VALUES (1, zeroblob(65536));
UPDATE items SET name = 'migrating' WHERE id = 1;
''',
};

void check(String message, {required bool condition}) {
  if (!condition) throw StateError(message);
}

Future<void> expectStorageBaseline(Executor db, String history) async {
  final rows = (await db.fetchAll(
    'SELECT id, name, length(payload) AS size FROM items ORDER BY id',
    [],
    (row) => [
      row.read<int>('id'),
      row.read<String>('name'),
      row.read<int>('size'),
    ],
  )).unwrap();
  check(
    'partial application data: $rows',
    condition:
        jsonEncode(rows) ==
        jsonEncode([
          [1, 'baseline 🍜', 65536],
        ]),
  );
  check(
    'migration history changed',
    condition: await readHistory(db) == history,
  );
  check(
    'original BLOB bytes changed',
    condition:
        (await db.fetchScalar<int>(
          "SELECT payload = CAST(replace(hex(zeroblob(32768)), '00', 'ab') AS BLOB) FROM items WHERE id = 1",
          [],
        )).unwrap() ==
        1,
  );
  check(
    'partial schema',
    condition:
        (await db.fetchScalar<int>(
          "SELECT COUNT(*) FROM sqlite_schema WHERE name = 'extra'",
          [],
        )).unwrap() ==
        0,
  );
  await expectStorageIntegrity(db);
}

Future<void> expectStorageIntegrity(Executor db) async => check(
  'integrity failed',
  condition:
      (await db.fetchScalar<String>('PRAGMA integrity_check', [])).unwrap() ==
      'ok',
);

Future<void> expectStorageReusable(Pool db) async {
  (await db.transaction<Unit>(
    (tx) async => (await queryExecute(
      "INSERT INTO items VALUES (2, 'reused', NULL)",
      [],
    ).execute(tx)).map((_) => unit),
  )).unwrap();
  check(
    'reuse failed',
    condition:
        (await db.fetchScalar<int>(
          'SELECT COUNT(*) FROM items',
          [],
        )).unwrap() ==
        2,
  );
}

Future<String> readHistory(Executor db) async => jsonEncode(
  (await db.fetchAll(
    'SELECT name, applied_at, checksum, checksum_source FROM __dust_schema_migrations ORDER BY name',
    [],
    (row) => [
      for (final name in ['name', 'applied_at', 'checksum', 'checksum_source'])
        row.read<String>(name),
    ],
  )).unwrap(),
);
