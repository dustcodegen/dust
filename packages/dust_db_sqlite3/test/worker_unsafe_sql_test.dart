import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  test(
    'worker unsafe adapter fetches, decodes and executes asynchronously',
    () async {
      final db = await Sqlite3WorkerDriver.open(':memory:');
      addTearDown(db.close);
      final unsafe = Sqlite3WorkerUnsafeSql(db);
      expectOk(await unsafe.execute('CREATE TABLE entries (id INTEGER)', []));
      expect(
        expectOk(
          await unsafe.execute('INSERT INTO entries VALUES (?)', [7]),
        ).rowsAffected,
        1,
      );
      final rows = expectOk(await unsafe.fetch('SELECT id FROM entries', []));
      expect(rows.single.read<int>('id'), 7);
      expect(
        expectOk(
          await unsafe.fetchAs(
            'SELECT id FROM entries',
            [],
            (row) => row.read<int>('id'),
          ),
        ),
        [7],
      );
      expect(
        expectOk(await unsafe.fetch('SELECT id FROM entries WHERE 0', [])),
        isEmpty,
      );
      expect(
        (await unsafe.fetch(
          'SELECT missing FROM entries',
          [],
        )).unwrapErr().category,
        SqlxErrorCategory.query,
      );
      expect(
        (await unsafe.fetchAs<int>(
          'SELECT id FROM entries',
          [],
          (_) => throw StateError('mapping'),
        )).unwrapErr().category,
        SqlxErrorCategory.decode,
      );
      final error = SqlxError.decode('local mapper');
      expect(
        (await unsafe.fetchAs<int>(
          'SELECT id FROM entries',
          [],
          (_) => throw error,
        )).unwrapErr(),
        same(error),
      );
      (await db.transaction<Unit>((tx) async {
        expect(
          (await unsafe.fetch(
            'SELECT id FROM entries',
            [],
          )).unwrapErr().category,
          SqlxErrorCategory.transaction,
        );
        expect((await unsafe.execute('DELETE FROM entries', [])).isErr, isTrue);
        return const Ok(unit);
      })).unwrap();
      expectOk(await db.close());
      expect(
        (await unsafe.fetch('SELECT 1', [])).unwrapErr().category,
        SqlxErrorCategory.connection,
      );
    },
  );
}
