import 'dart:convert';
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

  Future<void> expectBaseline(Executor db) async {
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
    await expectIntegrity(db);
  }

  Future<void> expectIntegrity(Executor db) async => check(
    'integrity failed',
    condition:
        (await db.fetchScalar<String>('PRAGMA integrity_check', [])).unwrap() ==
        'ok',
  );

  Future<void> expectReusable(Pool db) async {
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
