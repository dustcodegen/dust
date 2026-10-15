// Copyright (c) 2026 Ye Lin Aung
// SPDX-License-Identifier: MIT

import 'dart:isolate';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show
        initializeSqlite3Connection,
        openSqlite3WorkerConnection,
        runSqlite3Worker;
import 'package:flutter/scheduler.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:sqlite3/sqlite3.dart' as sqlite;

import 'probe_metrics.dart';

void Function(List<Object?>) _waitingWorker(SendPort notify) => (args) {
      runSqlite3Worker(args, open: (options, migrations) {
        final native = sqlite.sqlite3.open(options.path!);
        final wait = Stopwatch();
        try {
          native.busyHandler = (count) {
            if (count == 0) {
              wait
                ..reset()
                ..start();
              notify.send(null);
            }
            // Test-only wall-clock bound; native timeout durations are not a handshake.
            return wait.elapsed < const Duration(seconds: 5);
          };
        } catch (_) {
          native.close();
          rethrow;
        }
        return initializeSqlite3Connection(native, options, migrations);
      });
    };

/// Confirms native migration lock contention while actual UI frames progress.
Future<Map<String, Object?>> probeNativeWait(
    String path, ProbeMetrics metrics) async {
  final locker = Sqlite3Driver.open(
    path,
    options: const SqliteConnectOptions(journalMode: SqliteJournalMode.wal),
  );
  final busy = ReceivePort();
  final contended = busy.first;
  Sqlite3WorkerDriver? worker;
  (Object, StackTrace)? failure;
  var released = true;
  var opening = Future<void>.value();
  final watch = Stopwatch()..start();
  try {
    locker.database.execute('BEGIN IMMEDIATE');
    released = false;
    opening = openSqlite3WorkerConnection(
      SqliteConnectOptions.path(path, journalMode: SqliteJournalMode.wal),
      const {
        '0001.sql':
            'CREATE TABLE controlled_migration (id INTEGER PRIMARY KEY);'
      },
      entrypoint: _waitingWorker(busy.sendPort),
    ).then<void>(
      (value) => worker = value,
      onError: (Object error, StackTrace stack) {
        failure = (error, stack);
      },
    );
    await contended.timeout(const Duration(seconds: 5));
    final before = metrics.uiFrames;
    for (var frame = 0; frame < 3; frame++) {
      await SchedulerBinding.instance.endOfFrame
          .timeout(const Duration(seconds: 2));
    }
    final frames = metrics.uiFrames - before;
    expect(frames, greaterThanOrEqualTo(3));
    expect(worker, isNull,
        reason: 'Opening must still be waiting on the native lock.');
    locker.database.execute('ROLLBACK');
    released = true;
    await opening;
    if (failure != null) Error.throwWithStackTrace(failure!.$1, failure!.$2);
    expect(
        (await queryScalar<int>(
          'SELECT COUNT(*) FROM __dust_schema_migrations',
          [],
        ).fetchOne(worker!))
            .unwrap(),
        1);
    return {'openUs': watch.elapsedMicroseconds, 'uiFramesWhileLocked': frames};
  } finally {
    try {
      if (!released) locker.database.execute('ROLLBACK');
    } finally {
      await opening;
      try {
        if (worker != null) (await worker!.close()).unwrap();
      } finally {
        try {
          (await locker.close()).unwrap();
        } finally {
          busy.close();
        }
      }
    }
  }
}
