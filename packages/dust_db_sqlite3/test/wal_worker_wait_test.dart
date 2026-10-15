import 'dart:isolate';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show openSqlite3WorkerConnection, runSqlite3Worker;
import 'package:test/test.dart';

import 'support/expect_ok.dart';
import 'support/wal_fixture.dart';

// Only the port crosses isolates; the native handle stays with its owner.
void Function(List<Object?>) _notifyBusy(SendPort notify) => (arguments) {
  runSqlite3Worker(
    arguments,
    open: (options, migrations) {
      final db = Sqlite3Driver.connect(options, migrations: migrations);
      final wait = Stopwatch();
      db.database.busyHandler = (count) {
        if (count == 0) {
          wait.reset();
          wait.start();
          notify.send(null);
        }
        // Test-only bounded native wait, with a real lock and no guessed sleeps.
        return wait.elapsed < const Duration(seconds: 10);
      };
      return db;
    },
  );
};

void main() {
  for (final checkpointing in [false, true]) {
    test(
      'worker stays responsive during native WAL wait / checkpoint=$checkpointing',
      () async {
        final fixture = WalFixture();
        await fixture.seed();
        final writer = await fixture.open(background: false);
        final notified = ReceivePort();
        addTearDown(notified.close);
        final worker = await openSqlite3WorkerConnection(
          SqliteConnectOptions.path(
            fixture.path,
            journalMode: SqliteJournalMode.wal,
            synchronous: SqliteSynchronousMode.full,
            pragmas: const {'wal_autocheckpoint': 0},
          ),
          null,
          entrypoint: _notifyBusy(notified.sendPort),
        );
        addTearDown(() async => expectOk(await worker.close()));
        final busy = notified.first;
        var pending = Future<void>.value();
        var settled = false;
        try {
          expectOk(
            await writer.transaction<Unit>((tx) async {
              await writeValue(tx, 2);
              pending =
                  (checkpointing
                          ? checkpoint(
                              worker,
                              'TRUNCATE',
                            ).then((value) => expect(value, (0, 0, 0)))
                          : writeValue(worker, 3))
                      .whenComplete(() => settled = true);
              await busy.timeout(const Duration(seconds: 5));
              // The busy-handler handshake proves SQLite reached the real lock.
              await Future<void>(() {});
              expect(settled, isFalse);
              return const Ok(unit);
            }),
          );
        } finally {
          await pending;
        }
        await fixture.expectHealthy(worker, checkpointing ? 2 : 3);
        await writeValue(writer, 4);
        await fixture.expectHealthy(worker, 4);
      },
    );
  }
}
