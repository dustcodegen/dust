import 'dart:convert';
import 'dart:isolate';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shopping_app/core/db/shopping_cache_database.dart';

import 'probe_metrics.dart';
import 'probe_wait.dart';

/// Uses the existing generated async opener, not a replacement test facade.
Future<ShoppingCacheDatabase> openProbe(
  String path, {
  Duration busyTimeout = const Duration(seconds: 3),
}) =>
    ShoppingCacheDatabase.openBackground(
      path,
      options: SqliteConnectOptions(
        journalMode: SqliteJournalMode.wal,
        synchronous: SqliteSynchronousMode.full,
        busyTimeout: busyTimeout,
      ),
    );

/// Exact history, including timestamps and checksums, survives reopen/relaunch.
Future<String> probeHistory(ShoppingCacheDatabase db) async => jsonEncode(
      (await db.unsafe.fetch(
        'SELECT name, applied_at, checksum, checksum_source FROM __dust_schema_migrations ORDER BY name',
        [],
      ))
          .unwrap()
          .map(
            (row) => [
              for (final key in [
                'name',
                'applied_at',
                'checksum',
                'checksum_source'
              ])
                row.read<String>(key),
            ],
          )
          .toList(),
    );

/// Checks generated timeout/retry before the instrumented native wait probe.
Future<Map<String, Object?>> probeLockedOpening(
  String path,
  ProbeMetrics metrics,
) async {
  final locker = Sqlite3Driver.open(
    path,
    options: const SqliteConnectOptions(journalMode: SqliteJournalMode.wal),
  );
  try {
    locker.database.execute('BEGIN IMMEDIATE');
    try {
      ShoppingCacheDatabase? unexpected;
      try {
        unexpected = await openProbe(
          path,
          busyTimeout: const Duration(milliseconds: 200),
        );
        fail('locked migration unexpectedly succeeded');
      } on SqlxError catch (error) {
        expect(error.operation, 'BEGIN IMMEDIATE');
        expect(error.category, SqlxErrorCategory.migration);
        expect(error.cause.toString(), contains('SqliteException(5)'));
      } finally {
        if (unexpected != null) (await unexpected.close()).unwrap();
      }
      expect(
        locker.database
            .select(
              "SELECT COUNT(*) FROM sqlite_schema WHERE name = '__dust_schema_migrations'",
            )
            .single
            .values
            .single,
        0,
      );
    } finally {
      locker.database.execute('ROLLBACK');
    }

    final opened = await openProbe(path);
    try {
      expect(jsonDecode(await probeHistory(opened)), hasLength(2));
    } finally {
      (await opened.close()).unwrap();
    }
  } finally {
    (await locker.close()).unwrap();
  }
  return probeNativeWait('$path.wait', metrics);
}

/// Commits bounded test rows through the normal transaction API.
Future<void> seedProbe(ShoppingCacheDatabase db, String run) async {
  (await db.transaction<Unit>((tx) async {
    for (var id = 1; id <= 128; id++) {
      (await queryExecute(
        'INSERT INTO wishlist_cache (product_id, title, saved_at) VALUES (?, ?, ?)',
        [id, '$run-$id', '2026-10-15T00:00:00Z'],
      ).execute(tx))
          .unwrap();
    }
    return const Ok(unit);
  }))
      .unwrap();
}

/// Validates committed rows and proves row mapping runs on the caller isolate.
Future<void> verifyProbe(ShoppingCacheDatabase db, String run) async {
  final caller = Isolate.current;
  final rows = (await queryAs<(int, String)>(
    'SELECT product_id, title FROM wishlist_cache ORDER BY product_id LIMIT 32',
    [],
  ).fetchAllWith(db.connection, (row) {
    expect(Isolate.current, caller);
    return (row.read<int>('product_id'), row.read<String>('title'));
  }))
      .unwrap();
  expect(rows, [for (var id = 1; id <= 32; id++) (id, '$run-$id')]);
  expect(
    (await queryScalar<int>(
      'SELECT COUNT(*) FROM wishlist_cache',
      [],
    ).fetchOne(db.connection))
        .unwrap(),
    128,
  );
  expect(
    (await db.unsafe.fetch('PRAGMA integrity_check', []))
        .unwrap()
        .single
        .read<String>('integrity_check'),
    'ok',
  );
}
