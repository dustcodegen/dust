import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

import 'expect_ok.dart';

final class WalFixture {
  final Directory directory = Directory.systemTemp.createTempSync('dust-wal-');
  late final path = '${directory.path}/app.db';

  Future<void> seed() async {
    addTearDown(() => directory.deleteSync(recursive: true));
    final db = await open(background: false);
    expectOk(
      await db.execute('CREATE TABLE items (value INTEGER NOT NULL)', []),
    );
    expectOk(await db.execute('INSERT INTO items VALUES (1)', []));
    expectOk(await db.close());
  }

  Future<Pool> open({
    required bool background,
    Duration timeout = Duration.zero,
  }) async {
    final options = SqliteConnectOptions(
      journalMode: SqliteJournalMode.wal,
      synchronous: SqliteSynchronousMode.full,
      busyTimeout: timeout,
      pragmas: const {'wal_autocheckpoint': 0},
    );
    final db = background
        ? await Sqlite3WorkerDriver.open(path, options: options)
        : Sqlite3Driver.open(path, options: options);
    addTearDown(() async => expectOk(await db.close()));
    expect(
      expectOk(await db.fetchScalar<String>('PRAGMA journal_mode', [])),
      'wal',
    );
    expect(
      expectOk(await db.fetchScalar<int>('PRAGMA busy_timeout', [])),
      timeout.inMilliseconds,
    );
    return db;
  }

  Future<void> expectHealthy(Executor db, int value) async {
    expect(await readValue(db), value);
    expect(
      expectOk(await db.fetchScalar<String>('PRAGMA integrity_check', [])),
      'ok',
    );
  }
}

Future<int> readValue(Executor db) async => expectOk(
  await queryScalar<int>('SELECT value FROM items', []).fetchOne(db),
);

Future<void> writeValue(Executor db, int value) async => expectOk(
  await queryExecute('UPDATE items SET value = ?', [value]).execute(db),
);

Future<(int, int, int)> checkpoint(Executor db, String mode) async => expectOk(
  await db.fetchOne(
    'PRAGMA wal_checkpoint($mode)',
    [],
    (row) => (
      row.read<int>('busy'),
      row.read<int>('log'),
      row.read<int>('checkpointed'),
    ),
  ),
);

void expectBusy(SqlxError error, {required bool background, int extended = 5}) {
  expect(error.driver, Driver.sqlite3);
  expect(error.category, SqlxErrorCategory.query);
  if (background) {
    expect(error.cause, contains('SqliteException($extended)'));
    expect(error.cause, contains('(code $extended)'));
  } else {
    expect(
      error.cause,
      isA<sqlite.SqliteException>()
          .having((e) => e.resultCode, 'SQLITE_BUSY', 5)
          .having((e) => e.extendedResultCode, 'extended code', extended),
    );
  }
}
