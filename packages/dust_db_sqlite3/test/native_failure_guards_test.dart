import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show initializeSqlite3Connection;
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  test(
    'failed root BEGIN does not roll back a caller-owned transaction',
    () async {
      final db = Sqlite3Driver.open(':memory:');
      addTearDown(db.close);
      db.database.execute('BEGIN');
      var called = false;

      final error = _error(
        await db.transaction<Unit>((tx) async {
          called = true;
          return const Ok(unit);
        }),
      );

      _expectNativeError(error, SqlxErrorCategory.transaction, 'BEGIN');
      expect(called, isFalse);
      expect(db.database.autocommit, isFalse);
      db.database.execute('ROLLBACK');
      expectOk(await db.transaction<Unit>((tx) async => const Ok(unit)));
    },
  );

  test('failed nested SAVEPOINT does not invoke its callback', () async {
    final db = Sqlite3Driver.open(':memory:');
    addTearDown(db.close);
    var called = false;
    late SqlxError nestedError;

    final result = await db.transaction<Unit>((tx) async {
      db.database.close();
      final nested = await tx.transaction<Unit>((inner) async {
        called = true;
        return const Ok(unit);
      });
      nestedError = _error(nested);
      return nested;
    });

    expect(called, isFalse);
    expect(nestedError.category, SqlxErrorCategory.transaction);
    expect(nestedError.driver, Driver.sqlite3);
    expect(nestedError.operation, 'SAVEPOINT _dust_tx_1');
    expect(nestedError.cause, isA<StateError>());
    expect(_error(result).operation, 'ROLLBACK');
  });

  test('native pragma failure retains connection operation and cause', () {
    final native = sqlite.sqlite3.openInMemory()..close();
    expect(
      () => initializeSqlite3Connection(
        native,
        const SqliteConnectOptions.memory(foreignKeys: true),
        const {},
      ),
      throwsA(
        isA<SqlxError>()
            .having((e) => e.category, 'category', SqlxErrorCategory.connection)
            .having((e) => e.driver, 'driver', Driver.sqlite3)
            .having((e) => e.operation, 'operation', 'PRAGMA foreign_keys = ON')
            .having((e) => e.cause, 'cause', isA<StateError>()),
      ),
    );
  });

  test(
    'native close failure is returned once and driver rejects later work',
    () async {
      final native = _FailingCloseDatabase();
      final db = initializeSqlite3Connection(
        native,
        const SqliteConnectOptions.memory(),
        const {},
      );
      final error = _error(await db.close());
      expect(error.category, SqlxErrorCategory.connection);
      expect(error.driver, Driver.sqlite3);
      expect(error.operation, 'close');
      expect(error.cause, same(native.failure));
      expect(native.calls, 1);
      expectOk(await db.close());
      expect(native.calls, 1);
      expect(
        _error(await db.execute('SELECT 1', const [])).operation,
        'checkOpen',
      );
    },
  );
}

SqlxError _error<T>(Result<T, SqlxError> result) {
  expect(result, isA<Err<T, SqlxError>>());
  return result.match(ok: (_) => fail('expected failure'), err: (e) => e);
}

void _expectNativeError(
  SqlxError error,
  SqlxErrorCategory category,
  String operation,
) {
  expect(error.category, category);
  expect(error.driver, Driver.sqlite3);
  expect(error.operation, operation);
  expect(error.cause, isA<sqlite.SqliteException>());
}

final class _FailingCloseDatabase implements sqlite.Database {
  final failure = StateError('native close failed');
  var calls = 0;

  @override
  void close() {
    calls++;
    throw failure;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'Unexpected native operation: ${invocation.memberName}',
  );
}
