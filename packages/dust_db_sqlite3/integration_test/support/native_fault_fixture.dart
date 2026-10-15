import 'dart:ffi';
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

Sqlite3Driver _openFault(
  SqliteConnectOptions options,
  Map<String, String>? migrations,
) => initializeSqlite3Connection(
  sqlite.sqlite3.open(options.path!, vfs: 'dust_fault'),
  options,
  migrations,
);

void _worker(List<Object?> args) => runSqlite3Worker(args, open: _openFault);

final class NativeFaultFixture {
  NativeFaultFixture(this.path, this.journal, {required this.background});
  final String path;
  final String journal;
  final bool background;
  late sqlite.Database probe;
  late String history;
  int generation = 0;

  SqliteConnectOptions get options => SqliteConnectOptions.path(
    path,
    journalMode: SqliteJournalMode.values.byName(journal),
    synchronous: SqliteSynchronousMode.full,
    pragmas: const {
      'cache_size': 8,
      'cache_spill': 1,
      'wal_autocheckpoint': 0,
      'mmap_size': 0,
    },
  );

  Future<void> seed(String library) async {
    probe = sqlite.sqlite3.openInMemory();
    DynamicLibrary.open(library);
    final db = Sqlite3Driver.connect(options, migrations: initialMigrations);
    try {
      history = await readHistory(db);
    } finally {
      (await db.close()).unwrap();
    }
  }

  Future<Pool> open({Map<String, String>? migrations}) async => background
      ? await openSqlite3WorkerConnection(
          options,
          migrations,
          entrypoint: _worker,
        )
      : _openFault(options, migrations);

  void arm(
    String operation,
    String target, {
    required bool persistent,
    required bool partial,
  }) {
    final kind = [
      'none',
      'read',
      'write',
      'truncate',
      'sync',
    ].indexOf(operation);
    final file = ['none', 'database', 'journal', 'wal'].indexOf(target);
    check(
      'invalid fault: $operation / $target',
      condition: kind >= 0 && file >= 0,
    );
    if (kind != 0) {
      File('$path.control.hits').writeAsStringSync('0 0 0', flush: true);
    }
    File('$path.control.pending')
      ..writeAsStringSync(
        '${++generation} $kind $file ${persistent ? 1 : 0} ${partial ? 1 : 0}',
        flush: true,
      )
      ..renameSync('$path.control');
  }

  List<int> get evidence => File(
    '$path.control.hits',
  ).readAsStringSync().split(' ').map(int.parse).toList();
  int get hits => evidence[0];

  void disable() => arm('none', 'none', persistent: false, partial: false);

  void expectError(SqlxError error, String operation) {
    final extended = switch (operation) {
      'read' => 266,
      'write' => 778,
      'truncate' => 1546,
      'sync' => 1034,
      _ => throw ArgumentError(operation),
    };
    check('wrong driver: $error', condition: error.driver == Driver.sqlite3);
    check(
      'missing operation: $error',
      condition: error.operation != null && error.operation!.isNotEmpty,
    );
    if (background) {
      check(
        'wrong worker error: $error',
        condition: error.cause.toString().contains(
          'SqliteException($extended)',
        ),
      );
    } else {
      final cause = error.cause;
      check(
        'wrong native error: $error',
        condition:
            cause is sqlite.SqliteException &&
            cause.resultCode == 10 &&
            cause.extendedResultCode == extended,
      );
    }
    check('fault was never reached', condition: hits > 0);
  }
}
