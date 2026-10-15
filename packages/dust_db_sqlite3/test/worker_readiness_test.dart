import 'dart:isolate';

import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show openSqlite3WorkerConnection, runSqlite3Worker;
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

import 'support/expect_ok.dart';
import 'support/migration_fixture.dart';

void _instrumentedWorker(List<Object?> arguments) => runSqlite3Worker(
  arguments,
  open: (options, migrations) {
    final db = Sqlite3Driver.connect(options, migrations: migrations);
    db.database.createFunction(
      functionName: 'worker_isolate',
      argumentCount: const sqlite.AllowedArgumentCount(0),
      function: (_) => Isolate.current.hashCode,
    );
    return db;
  },
);

// This closure captures only a SendPort, never the caller's native resources.
void Function(List<Object?>) _notifyOpening(SendPort notify) => (arguments) {
  notify.send(null);
  runSqlite3Worker(arguments);
};

void main() {
  test(
    'SQL runs on one persistent worker while mappers stay on the caller',
    () async {
      final db = await openSqlite3WorkerConnection(
        const SqliteConnectOptions.memory(),
        null,
        entrypoint: _instrumentedWorker,
      );
      addTearDown(db.close);
      final worker = expectOk(
        await db.fetchScalar<int>('SELECT worker_isolate()', []),
      );
      expect(worker, isNot(Isolate.current.hashCode));
      final value = expectOk(
        await db.fetchOne<int>('SELECT worker_isolate() AS id', [], (row) {
          expect(Isolate.current.hashCode, isNot(worker));
          return row.read<int>('id');
        }),
      );
      expect(value, worker);
    },
  );

  test(
    'native migration lock waiting does not block the caller event loop',
    () async {
      final fixture = MigrationFixture();
      addTearDown(fixture.dispose);
      expectOk(await fixture.open(migrations: firstMigration).close());
      final writer = sqlite.sqlite3.open(fixture.path)
        ..execute('BEGIN IMMEDIATE');
      final notify = ReceivePort();
      addTearDown(notify.close);
      var settled = false;
      final opening = openSqlite3WorkerConnection(
        SqliteConnectOptions.path(
          fixture.path,
          busyTimeout: const Duration(seconds: 10),
        ),
        upgradedMigrations,
        entrypoint: _notifyOpening(notify.sendPort),
      )..then((_) => settled = true);
      try {
        await notify.first;
        await Future<void>(() {});
        expect(settled, isFalse);
      } finally {
        writer.execute('COMMIT');
        writer.close();
      }
      final db = await opening;
      try {
        expect(
          expectOk(
            await db.fetchScalar<int>(
              'SELECT COUNT(*) FROM __dust_schema_migrations',
              [],
            ),
          ),
          2,
        );
        expect(
          expectOk(
            await db.fetchScalar<int>(
              "SELECT COUNT(*) FROM pragma_table_info('items') WHERE name = 'active'",
              [],
            ),
          ),
          1,
        );
      } finally {
        expectOk(await db.close());
      }
    },
  );

  test(
    'startup snapshots mutable options and migrations before spawning',
    () async {
      final pragmas = <String, Object>{'cache_size': -321};
      final migrations = {'0001.sql': 'CREATE TABLE entries (id INTEGER);'};
      final opening = Sqlite3WorkerDriver.open(
        ':memory:',
        options: SqliteConnectOptions(
          busyTimeout: const Duration(milliseconds: 123),
          foreignKeys: true,
          pragmas: pragmas,
          journalMode: SqliteJournalMode.memory,
          synchronous: SqliteSynchronousMode.full,
        ),
        migrations: migrations,
      );
      pragmas['cache_size'] = -999;
      migrations['0001.sql'] = 'INVALID SQL;';
      final db = await opening;
      try {
        expect(
          expectOk(await db.fetchScalar<int>('PRAGMA cache_size', [])),
          -321,
        );
        expect(
          expectOk(await db.fetchScalar<int>('PRAGMA busy_timeout', [])),
          123,
        );
        expect(
          expectOk(await db.fetchScalar<int>('PRAGMA foreign_keys', [])),
          1,
        );
        expect(
          expectOk(await db.fetchScalar<String>('PRAGMA journal_mode', [])),
          'memory',
        );
        expect(
          expectOk(await db.fetchScalar<int>('PRAGMA synchronous', [])),
          2,
        );
        expectOk(await db.execute('INSERT INTO entries VALUES (7)', []));
      } finally {
        expectOk(await db.close());
      }
    },
  );
}
