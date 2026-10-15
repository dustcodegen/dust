import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show
        initializeSqlite3Connection,
        openSqlite3WorkerConnection,
        runSqlite3Worker;
import 'package:sqlite3/sqlite3.dart' as sqlite;

import 'crash_schema.dart';

// The parent keeps stdin open and kills this process after reading the marker.
void _pause(String marker) {
  File('$marker.pending')
    ..writeAsStringSync('ready', flush: true)
    ..renameSync(marker);
  stdin.readByteSync();
  throw StateError('crash child resumed instead of being killed');
}

Sqlite3Driver _migrationOpen(
  SqliteConnectOptions options,
  Map<String, String>? migrations,
  String marker,
  String stage,
) {
  final native = sqlite.sqlite3.open(options.path!);
  if (stage == 'migration-body') {
    native.updatesSync.listen((update) {
      if (update.tableName == 'items' &&
          update.kind == sqlite.SqliteUpdateKind.update) {
        _pause(marker);
      }
    });
  } else {
    // Dust invokes an existing commit filter only at its final owned COMMIT.
    native.commitFilter = () {
      _pause(marker);
      return true;
    };
  }
  return initializeSqlite3Connection(native, options, migrations);
}

Future<void> main(List<String> args) async {
  final [path, marker, journal, mode, stage] = args;
  final background = mode == 'worker';
  final options = crashOptions(journal);
  final migrating = stage.startsWith('migration-');
  final migrations = migrating
      ? crashMigrations
      : {'0001.sql': crashMigrations['0001.sql']!};
  if (migrating && stage != 'migration-committed') {
    final pathOptions = SqliteConnectOptions.path(
      path,
      journalMode: options.journalMode,
      synchronous: options.synchronous,
      busyTimeout: options.busyTimeout,
      pragmas: options.pragmas,
    );
    if (background) {
      await openSqlite3WorkerConnection(
        pathOptions,
        migrations,
        entrypoint: (arguments) => runSqlite3Worker(
          arguments,
          open: (options, migrations) =>
              _migrationOpen(options, migrations, marker, stage),
        ),
      );
    } else {
      _migrationOpen(pathOptions, migrations, marker, stage);
    }
    throw StateError('migration did not reach its crash boundary');
  }
  final db = background
      ? await Sqlite3WorkerDriver.open(
          path,
          options: options,
          migrations: migrations,
        )
      : Sqlite3Driver.open(path, options: options, migrations: migrations);
  if (!migrating) {
    (await db.transaction<Unit>((tx) async {
      (await queryExecute(
        "UPDATE items SET name = 'changed' WHERE id = 1",
        [],
      ).execute(tx)).unwrap();
      (await queryExecute(
        "INSERT INTO items VALUES (2, 'pending', zeroblob(262144))",
        [],
      ).execute(tx)).unwrap();
      if (stage == 'nested') {
        return tx.transaction<Unit>((child) async {
          (await queryExecute(
            "INSERT INTO items VALUES (3, 'nested', zeroblob(262144))",
            [],
          ).execute(child)).unwrap();
          _pause(marker);
          return const Ok(unit);
        });
      }
      if (stage == 'outer') _pause(marker);
      return const Ok(unit);
    })).unwrap();
  }
  _pause(marker);
}
