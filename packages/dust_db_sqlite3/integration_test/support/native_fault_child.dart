import 'dart:convert';

import 'package:dust_dart/db.dart';

import 'native_fault_fixture.dart';

const updateSql =
    "UPDATE items SET name = 'pending', payload = zeroblob(131072) WHERE id = 1";
const checkpointSql = 'PRAGMA wal_checkpoint(TRUNCATE)';
const readSql = 'SELECT length(payload) FROM items';

Future<void> main(List<String> args) async {
  final [
    library,
    path,
    journal,
    mode,
    stage,
    operation,
    target,
    persistence,
    writeKind,
  ] = args;
  final fixture = NativeFaultFixture(
    path,
    journal,
    background: mode == 'worker',
  );
  await fixture.seed(library);
  try {
    switch (stage) {
      case 'transaction' || 'nested':
        final db = await fixture.open();
        try {
          var callbacks = 0;
          final result = await db.transaction<Unit>((tx) async {
            callbacks++;
            fixture.arm(
              operation,
              target,
              persistent: persistence == 'persistent',
              partial: writeKind == 'partial',
            );
            Future<Result<Unit, SqlxError>> update(
              Transaction current,
            ) async => (await queryExecute(
              updateSql,
              [],
            ).execute(current)).map((_) => unit);
            return stage == 'nested'
                ? tx.transaction<Unit>(update)
                : update(tx);
          });
          fixture.expectError(result.unwrapErr(), operation);
          final error = result.unwrapErr();
          check(
            'original statement/control context lost: $error',
            condition:
                error.operation == updateSql || error.operation == 'COMMIT',
          );
          check('callback was replayed', condition: callbacks == 1);
        } finally {
          fixture.disable();
          (await db.close()).unwrap();
        }
      case 'migration':
        fixture.arm(
          operation,
          target,
          persistent: persistence == 'persistent',
          partial: writeKind == 'partial',
        );
        Pool? db;
        try {
          db = await fixture.open(migrations: pendingMigrations);
          throw StateError('migration falsely succeeded');
        } on SqlxError catch (error) {
          fixture.expectError(error, operation);
          check(
            'wrong migration context: $error',
            condition: error.category == SqlxErrorCategory.migration,
          );
          check(
            'migration operation lost: $error',
            condition:
                error.operation == '0002.sql' || error.operation == 'COMMIT',
          );
        } finally {
          fixture.disable();
          if (db != null) (await db.close()).unwrap();
        }
      case 'checkpoint':
        final db = await fixture.open();
        try {
          (await queryExecute(
            "UPDATE items SET name = 'committed' WHERE id = 1",
            [],
          ).execute(db)).unwrap();
          fixture.arm(
            operation,
            target,
            persistent: persistence == 'persistent',
            partial: writeKind == 'partial',
          );
          final result = await db.fetchAll(
            checkpointSql,
            [],
            (row) => row.read<int>('busy'),
          );
          fixture.expectError(result.unwrapErr(), operation);
          check(
            'checkpoint operation lost',
            condition: result.unwrapErr().operation == checkpointSql,
          );
          fixture.disable();
          check(
            'acknowledged commit lost',
            condition:
                (await db.fetchScalar<String>(
                  'SELECT name FROM items WHERE id = 1',
                  [],
                )).unwrap() ==
                'committed',
          );
          await fixture.expectIntegrity(db);
        } finally {
          fixture.disable();
          (await db.close()).unwrap();
        }
      case 'read':
        final db = await fixture.open();
        try {
          (await db.execute('PRAGMA shrink_memory', [])).unwrap();
          fixture.arm(
            operation,
            target,
            persistent: persistence == 'persistent',
            partial: false,
          );
          final result = await db.fetchScalar<int>(
            readSql,
            [],
          );
          fixture.expectError(result.unwrapErr(), operation);
          check(
            'read operation lost',
            condition: result.unwrapErr().operation == readSql,
          );
        } finally {
          fixture.disable();
          (await db.close()).unwrap();
        }
      default:
        throw ArgumentError(stage);
    }
    final db = await fixture.open();
    try {
      if (stage == 'checkpoint') {
        check(
          'acknowledged commit lost after reopen',
          condition:
              (await db.fetchScalar<String>(
                'SELECT name FROM items WHERE id = 1',
                [],
              )).unwrap() ==
              'committed',
        );
        (await queryExecute(
          "UPDATE items SET name = 'baseline 🍜' WHERE id = 1",
          [],
        ).execute(db)).unwrap();
      }
      await fixture.expectBaseline(db);
      await fixture.expectReusable(db);
      if (stage == 'migration') {
        (await db.close()).unwrap();
        final migrated = await fixture.open(migrations: pendingMigrations);
        late String appliedHistory;
        try {
          check(
            'migration retry failed',
            condition:
                (await migrated.fetchScalar<int>(
                  'SELECT COUNT(*) FROM extra',
                  [],
                )).unwrap() ==
                1,
          );
          await fixture.expectIntegrity(migrated);
          appliedHistory = await readHistory(migrated);
        } finally {
          (await migrated.close()).unwrap();
        }
        final repeated = await fixture.open(migrations: pendingMigrations);
        try {
          check(
            'migration history changed on repeated startup',
            condition: await readHistory(repeated) == appliedHistory,
          );
        } finally {
          (await repeated.close()).unwrap();
        }
      }
    } finally {
      (await db.close()).unwrap();
    }
    print(
      jsonEncode({
        'status': 'ok',
        'hits': fixture.hits,
        'partialWrites': fixture.evidence[1],
        'partialBytes': fixture.evidence[2],
        'engine': fixture.probe
            .select('SELECT sqlite_version()')
            .single
            .values
            .single,
        'sourceId': fixture.probe
            .select('SELECT sqlite_source_id()')
            .single
            .values
            .single,
      }),
    );
  } finally {
    fixture.disable();
    fixture.probe.close();
  }
}
