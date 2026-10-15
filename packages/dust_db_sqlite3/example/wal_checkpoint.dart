import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

/// Inspect blocked WAL checkpoint status; run with optional --background.
Future<void> main(List<String> args) async {
  final directory = Directory.systemTemp.createTempSync('dust-wal-example-');
  final connections = <Pool>[];
  try {
    Future<Pool> open() async {
      final path = '${directory.path}/app.db';
      const options = SqliteConnectOptions(
        journalMode: SqliteJournalMode.wal,
        synchronous: SqliteSynchronousMode.full,
        busyTimeout: Duration.zero,
        pragmas: {'wal_autocheckpoint': 0},
      );
      final db = args.contains('--background')
          ? await Sqlite3WorkerDriver.open(path, options: options)
          : Sqlite3Driver.open(path, options: options);
      connections.add(db);
      return db;
    }

    final reader = await open();
    (await reader.execute('CREATE TABLE items (value INTEGER)', [])).unwrap();
    (await queryExecute(
      'INSERT INTO items VALUES (1)',
      [],
    ).execute(reader)).unwrap();
    final writer = await open();
    Future<int> value(Executor db) async => (await queryScalar<int>(
      'SELECT value FROM items',
      [],
    ).fetchOne(db)).unwrap();
    Future<(int, int, int)> checkpoint() async => (await writer.fetchOne(
      'PRAGMA wal_checkpoint(TRUNCATE)',
      [],
      (row) => (
        row.read<int>('busy'),
        row.read<int>('log'),
        row.read<int>('checkpointed'),
      ),
    )).unwrap();
    (await reader.transaction<Unit>((tx) async {
      final original = await value(tx);
      (await queryExecute(
        'UPDATE items SET value = 2',
        [],
      ).execute(writer)).unwrap();
      final status = await checkpoint();
      if (status.$1 != 1 || await value(tx) != original) {
        throw StateError('expected a held snapshot and blocked checkpoint');
      }
      print('reader snapshot: $original, checkpoint busy: ${status.$1}');
      return const Ok(unit);
    })).unwrap();
    // Explicitly try again after the reader finishes, never replay a write.
    final status = await checkpoint();
    if (status != (0, 0, 0)) throw StateError('checkpoint incomplete: $status');
    print(
      'checkpoint complete: $status, current value: ${await value(reader)}',
    );
  } finally {
    try {
      for (final db in connections.reversed) {
        (await db.close()).unwrap();
      }
    } finally {
      directory.deleteSync(recursive: true);
    }
  }
}
