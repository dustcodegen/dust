import 'dart:convert';

import 'package:dust_dart/db.dart';

import 'filesystem_fixture.dart';
import 'storage_assertions.dart';

Future<void> main(List<String> args) async {
  final [path, journal, mode, stage, marker] = args;
  final fixture = FilesystemFixture(
    path,
    journal,
    marker,
    background: mode == 'worker',
  );
  await fixture.seed();
  late SqlxError failure;
  if (stage == 'migration') {
    Pool? unexpected;
    try {
      unexpected = await fixture.open(pending: true, pause: true);
      throw StateError('migration falsely succeeded');
    } on SqlxError catch (error) {
      fixture.expectFull(error, '0003.sql');
      check(
        'wrong migration category',
        condition: error.category == SqlxErrorCategory.migration,
      );
      failure = error;
    } finally {
      if (unexpected != null) (await unexpected.close()).unwrap();
    }
    pauseForParent(marker, 'failed', {
      'operation': failure.operation,
      'nativeCode': 13,
    });
  } else {
    final db = await fixture.open();
    try {
      if (stage == 'checkpoint' || stage == 'standalone') {
        if (stage == 'checkpoint') {
          (await queryExecute(largeInsert, []).execute(db)).unwrap();
        }
        pauseForParent(marker, 'ready');
        if (stage == 'checkpoint') {
          final result = await db.fetchAll(
            'PRAGMA wal_checkpoint(TRUNCATE)',
            [],
            (row) => row.read<int>('busy'),
          );
          failure = result.unwrapErr();
          fixture.expectFull(failure, 'PRAGMA wal_checkpoint(TRUNCATE)');
        } else {
          failure = (await queryExecute(
            largeInsert,
            [],
          ).execute(db)).unwrapErr();
          fixture.expectFull(failure, largeInsert);
        }
      } else {
        var callbacks = 0;
        final result = await db.transaction<Unit>((tx) async {
          callbacks++;
          (await queryExecute(
            "UPDATE items SET name = 'prefix' WHERE id = 1",
            [],
          ).execute(tx)).unwrap();
          pauseForParent(marker, 'ready');
          Future<Result<Unit, SqlxError>> failWrite(
            Transaction current,
          ) async => (await queryExecute(
            largeInsert,
            [],
          ).execute(current)).map((_) => unit);
          if (stage == 'nested') return tx.transaction<Unit>(failWrite);
          final failed = await failWrite(tx);
          fixture.expectFull(failed.unwrapErr(), largeInsert);
          if (stage == 'ignored') {
            final following = await tx.execute(
              "INSERT INTO items VALUES (4, 'must not commit', NULL)",
              [],
            );
            fixture.expectFull(following.unwrapErr(), largeInsert);
            return const Ok(unit);
          }
          return failed;
        });
        check('transaction callback replayed', condition: callbacks == 1);
        failure = result.unwrapErr();
        fixture.expectFull(failure, largeInsert);
      }
      pauseForParent(marker, 'failed', {
        'operation': failure.operation,
        'nativeCode': 13,
      });
    } finally {
      (await db.close()).unwrap();
    }
  }

  final db = await fixture.open();
  try {
    if (stage == 'checkpoint') {
      await expectLargeRow(db);
      (await db.execute('DELETE FROM items WHERE id = 3', [])).unwrap();
    }
    await expectStorageBaseline(db, fixture.history);
    if (stage == 'migration') {
      check(
        'earlier pending schema survived failure',
        condition:
            (await db.fetchScalar<int>(
              "SELECT COUNT(*) FROM sqlite_schema WHERE name = 'before_full'",
              [],
            )).unwrap() ==
            0,
      );
    }
    if (stage != 'migration') {
      (await db.transaction<Unit>(
        (tx) async =>
            (await queryExecute(largeInsert, []).execute(tx)).map((_) => unit),
      )).unwrap();
      await expectLargeRow(db);
      await expectStorageIntegrity(db);
    }
  } finally {
    (await db.close()).unwrap();
  }
  if (stage == 'migration') {
    final migrated = await fixture.open(pending: true);
    late String history;
    try {
      final rows = (await migrated.fetchScalar<int>(
        'SELECT COUNT(*) FROM extra',
        [],
      )).unwrap();
      check('explicit migration retry failed', condition: rows == 2);
      history = await readHistory(migrated);
      await expectStorageIntegrity(migrated);
    } finally {
      (await migrated.close()).unwrap();
    }
    final repeated = await fixture.open(pending: true);
    try {
      check(
        'migration repeated',
        condition: await readHistory(repeated) == history,
      );
    } finally {
      (await repeated.close()).unwrap();
    }
  }
  final probe = await fixture.open(pending: stage == 'migration');
  try {
    if (stage == 'migration') {
      check(
        'earlier pending data not recovered',
        condition:
            (await probe.fetchScalar<String>(
              'SELECT value FROM before_full',
              [],
            )).unwrap() ==
            'earlier pending file',
      );
      final rows = (await probe.fetchAll(
        'SELECT id, length(payload) AS size FROM extra ORDER BY id',
        [],
        (row) => [row.read<int>('id'), row.read<int>('size')],
      )).unwrap();
      check(
        'migrated data changed after reopening',
        condition:
            jsonEncode(rows) ==
            jsonEncode([
              [1, 64],
              [2, 262144],
            ]),
      );
      check(
        'migration BLOB bytes changed',
        condition:
            (await probe.fetchScalar<int>(
              'SELECT SUM(payload = zeroblob(length(payload))) FROM extra',
              [],
            )).unwrap() ==
            2,
      );
    } else {
      await expectLargeRow(probe);
      check(
        'history changed after explicit retry',
        condition: await readHistory(probe) == fixture.history,
      );
    }
    await expectStorageIntegrity(probe);
    print(
      jsonEncode({
        'status': 'ok',
        'stage': stage,
        'nativeCode': 13,
        'engine': (await probe.fetchScalar<String>(
          'SELECT sqlite_version()',
          [],
        )).unwrap(),
        'sourceId': (await probe.fetchScalar<String>(
          'SELECT sqlite_source_id()',
          [],
        )).unwrap(),
      }),
    );
  } finally {
    (await probe.close()).unwrap();
  }
}

Future<void> expectLargeRow(Executor db) async => check(
  'large committed BLOB lost',
  condition:
      (await db.fetchScalar<int>(
        "SELECT payload = CAST(replace(hex(zeroblob(131072)), '00', 'cd') AS BLOB) FROM items WHERE id = 3",
        [],
      )).unwrap() ==
      1,
);
