import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  test(
    'sqlite scalar fetch handles nullability, too many rows, and closed driver',
    () async {
      final pool = Sqlite3Driver.open(
        ':memory:',
        migrations: const <String, String>{
          '0001.sql': 'CREATE TABLE numbers (value INTEGER);',
        },
      );

      final emptyNullable = await pool.fetchScalar<int?>(
        'SELECT value FROM numbers',
        const [],
      );
      expect(emptyNullable.match(ok: (value) => value, err: (_) => -1), isNull);

      final emptyRequired = await pool.fetchScalar<int>(
        'SELECT value FROM numbers',
        const [],
      );
      expect(emptyRequired, isA<Err<int, SqlxError>>());

      expectOk(
        await queryExecute(r'INSERT INTO numbers (value) VALUES (?)', [
          1,
        ]).execute(pool),
      );
      expectOk(
        await queryExecute(r'INSERT INTO numbers (value) VALUES (?)', [
          2,
        ]).execute(pool),
      );

      final tooMany = await pool.fetchScalar<int>(
        'SELECT value FROM numbers ORDER BY value',
        const [],
      );
      expect(tooMany, isA<Err<int, SqlxError>>());

      final first = await pool.fetchScalar<int>(
        'SELECT value FROM numbers WHERE value = 1',
        const [],
      );
      final nullableValue = await pool.fetchScalar<int?>(
        'SELECT value FROM numbers WHERE value = 1',
        const [],
      );
      expect(first.match(ok: (value) => value, err: (_) => -1), 1);
      expect(nullableValue.match(ok: (value) => value, err: (_) => -1), 1);

      expectOk(
        await queryExecute(r'INSERT INTO numbers (value) VALUES (?)', [
          null,
        ]).execute(pool),
      );
      final nullRequired = await pool.fetchScalar<int>(
        'SELECT value FROM numbers WHERE value IS NULL',
        const [],
      );
      expect(nullRequired, isA<Err<int, SqlxError>>());

      final wrongType = await pool.fetchScalar<int>(
        'SELECT CAST(value AS TEXT) FROM numbers WHERE value = 1',
        const [],
      );
      expect(wrongType, isA<Err<int, SqlxError>>());
      final nullableWrongType = await pool.fetchScalar<int?>(
        'SELECT CAST(value AS TEXT) FROM numbers WHERE value = 1',
        const [],
      );
      expect(nullableWrongType, isA<Err<int?, SqlxError>>());

      final queryError = await pool.fetchScalar<int>(
        'SELECT value FROM missing_table',
        const [],
      );
      expect(queryError, isA<Err<int, SqlxError>>());

      await pool.close();
      await pool.close();

      final afterClose = await pool.fetchScalar<int>(
        'SELECT value FROM numbers',
        const [],
      );
      expect(afterClose, isA<Err<int, SqlxError>>());
    },
  );
}
