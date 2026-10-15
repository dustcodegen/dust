import 'dart:convert';
import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show
        initializeSqlite3Connection,
        openSqlite3WorkerConnection,
        runSqlite3Worker;
import 'package:sqlite3/sqlite3.dart' as sqlite;

import 'storage_assertions.dart';

const largeInsert =
    "INSERT INTO items VALUES (3, 'large', CAST(replace(hex(zeroblob(131072)), '00', 'cd') AS BLOB))";
const Map<String, String> filesystemMigrations = {
  ...initialMigrations,
  '0002.sql': '''
CREATE TABLE before_full (value TEXT NOT NULL);
INSERT INTO before_full VALUES ('earlier pending file');
''',
  '0003.sql': '''
CREATE TABLE extra (id INTEGER PRIMARY KEY, payload BLOB);
INSERT INTO extra VALUES (1, zeroblob(64));
UPDATE items SET name = 'migrating' WHERE id = 1;
SELECT filesystem_full_ready();
INSERT INTO extra VALUES (2, zeroblob(262144));
''',
};

/// The parent owns filling/restoring the verified volume, never this child.
void pauseForParent(String marker, String stage, [Object data = 'ready']) {
  File('$marker.$stage.pending')
    ..writeAsStringSync(jsonEncode(data), flush: true)
    ..renameSync('$marker.$stage');
  if (stdin.readByteSync() != 10) throw StateError('parent abandoned $stage');
}

Sqlite3Driver _migrationOpen(
  SqliteConnectOptions options,
  Map<String, String>? migrations,
  String marker, {
  required bool pause,
}) {
  final native = sqlite.sqlite3.open(options.path!);
  try {
    var historyWrites = 0;
    if (pause) {
      native.updatesSync.listen((update) {
        if (update.tableName == '__dust_schema_migrations' &&
            update.kind == sqlite.SqliteUpdateKind.insert) {
          historyWrites++;
        }
      });
    }
    native.createFunction(
      functionName: 'filesystem_full_ready',
      argumentCount: const sqlite.AllowedArgumentCount(0),
      function: (_) {
        if (pause) {
          check(
            'earlier pending history insert not observed',
            condition: historyWrites == 1,
          );
          pauseForParent(marker, 'ready');
        }
        return 0;
      },
    );
  } catch (_) {
    native.close();
    rethrow;
  }
  return initializeSqlite3Connection(native, options, migrations);
}

// Captures only path/state, never the caller's native database handle.
void Function(List<Object?>) _migrationWorker(
  String marker, {
  required bool pause,
}) =>
    (args) => runSqlite3Worker(
      args,
      open: (options, migrations) =>
          _migrationOpen(options, migrations, marker, pause: pause),
    );

final class FilesystemFixture {
  FilesystemFixture(
    this.path,
    this.journal,
    this.marker, {
    required this.background,
  });
  final String path;
  final String journal;
  final String marker;
  final bool background;
  late String history;

  SqliteConnectOptions get options => SqliteConnectOptions.path(
    path,
    journalMode: SqliteJournalMode.values.byName(journal),
    synchronous: SqliteSynchronousMode.full,
    busyTimeout: Duration.zero,
    pragmas: const {'cache_size': 8, 'cache_spill': 1, 'wal_autocheckpoint': 0},
  );

  Future<Pool> open({bool pending = false, bool pause = false}) async {
    if (pending) {
      return background
          ? await openSqlite3WorkerConnection(
              options,
              filesystemMigrations,
              entrypoint: _migrationWorker(marker, pause: pause),
            )
          : _migrationOpen(options, filesystemMigrations, marker, pause: pause);
    }
    return background
        ? await Sqlite3WorkerDriver.connect(
            options,
            migrations: initialMigrations,
          )
        : Sqlite3Driver.connect(options, migrations: initialMigrations);
  }

  Future<void> seed() async {
    final db = await open();
    try {
      history = await readHistory(db);
    } finally {
      (await db.close()).unwrap();
    }
  }

  void expectFull(SqlxError error, String operation) {
    check('wrong operation: $error', condition: error.operation == operation);
    check('wrong driver: $error', condition: error.driver == Driver.sqlite3);
    if (background) {
      check(
        'not native FULL: $error',
        condition: error.cause.toString().contains('SqliteException(13)'),
      );
    } else {
      final cause = error.cause;
      check(
        'not native FULL: $error',
        condition: cause is sqlite.SqliteException && cause.resultCode == 13,
      );
    }
  }
}
