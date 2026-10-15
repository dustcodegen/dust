import 'dart:convert';
import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

const _migrations = {
  '0001_notes.sql':
      'CREATE TABLE notes ('
      'body TEXT NOT NULL, payload BLOB NOT NULL, optional TEXT);',
};

/// Exercises public connection/transaction APIs against an owned temporary DB.
Future<Map<String, Object?>> probe({required bool background}) async {
  final directory = Directory.systemTemp.createTempSync('dust-sdk-consumer-');
  final path = '${directory.path}/notes.db';
  Future<Connection> open() async => background
      ? Sqlite3WorkerDriver.open(path, migrations: _migrations)
      : Sqlite3Driver.open(path, migrations: _migrations);
  Connection? db;
  try {
    db = await open();
    final inserted = (await db.transaction(
      (tx) => queryExecute(
        r'INSERT INTO notes VALUES ($1, $2, $3)',
        [
          'persisted',
          Uint8List.fromList([0, 127, 255]),
          null,
        ],
      ).execute(tx),
    )).unwrap();
    _require(inserted.rowsAffected == 1, 'Transaction did not insert one row.');
    final before = await _snapshot(db);
    (await db.close()).unwrap();
    db = null;
    db = await open();
    final after = await _snapshot(db);
    _require(jsonEncode(before) == jsonEncode(after), 'Reopen changed data.');
    return {'mode': background ? 'worker' : 'sync', ...after};
  } finally {
    if (db != null) (await db.close()).unwrap();
    directory.deleteSync(recursive: true);
  }
}

Future<Map<String, Object?>> _snapshot(Connection db) async {
  final caller = Isolate.current;
  final rows =
      (await queryAs<({String body, List<int> payload, String? optional})>(
            'SELECT body, payload, optional FROM notes',
            [],
          ).fetchAllWith(db, (row) {
            _require(
              Isolate.current == caller,
              'Mapper left the caller isolate.',
            );
            return (
              body: row.read<String>('body'),
              payload: row.read<Uint8List>('payload'),
              optional: row.readNullable<String>('optional'),
            );
          }))
          .unwrap();
  _require(rows.length == 1, 'Expected one persisted row.');
  final row = rows.single;
  _require(
    row.body == 'persisted' &&
        jsonEncode(row.payload) == '[0,127,255]' &&
        row.optional == null,
    'Typed/BLOB/nullable values changed.',
  );
  final migration =
      (await queryAs<({String checksum, String appliedAt})>(
            'SELECT name, checksum, checksum_source, applied_at '
            'FROM __dust_schema_migrations',
            [],
          ).fetchOneWith(db, (row) {
            _require(
              row.read<String>('name') == '0001_notes.sql',
              'Wrong migration.',
            );
            _require(
              row.read<String>('checksum_source') == 'applied',
              'Wrong source.',
            );
            final checksum = row.read<String>('checksum');
            _require(
              RegExp(r'^[a-f0-9]{64}$').hasMatch(checksum),
              'Invalid checksum.',
            );
            return (
              checksum: checksum,
              appliedAt: row.read<String>('applied_at'),
            );
          }))
          .unwrap();
  DateTime.parse(migration.appliedAt);
  final integrity = (await queryScalar<String>(
    'PRAGMA integrity_check',
    [],
  ).fetchOne(db)).unwrap();
  _require(integrity == 'ok', 'Integrity check failed: $integrity');
  return {
    'rows': rows.length,
    'integrity': integrity,
    'migrationChecksum': migration.checksum,
    'migrationAppliedAt': migration.appliedAt,
    'sqliteVersion': (await queryScalar<String>(
      'SELECT sqlite_version()',
      [],
    ).fetchOne(db)).unwrap(),
    'sqliteSourceId': (await queryScalar<String>(
      'SELECT sqlite_source_id()',
      [],
    ).fetchOne(db)).unwrap(),
  };
}

void _require(bool condition, String message) {
  // AOT disables assertions: failed proof must still exit nonzero.
  if (!condition) throw StateError(message);
}
