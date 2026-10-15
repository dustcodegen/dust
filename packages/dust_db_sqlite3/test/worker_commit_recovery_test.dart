import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  test('failed worker COMMIT rolls back and permits reuse', () async {
    final db = await Sqlite3WorkerDriver.connect(
      const SqliteConnectOptions.memory(foreignKeys: true),
      migrations: const {
        '0001.sql': '''
CREATE TABLE parents(id INTEGER PRIMARY KEY);
CREATE TABLE children(parent_id INTEGER REFERENCES parents(id) DEFERRABLE INITIALLY DEFERRED);
''',
      },
    );
    addTearDown(db.close);
    final result = await db.transaction<Unit>(
      (tx) async => (await tx.execute(
        'INSERT INTO children VALUES (1)',
        [],
      )).map((_) => unit),
    );
    expect(result.unwrapErr().operation, 'COMMIT');
    expect(result.unwrapErr().kind, SqlxErrorKind.foreignKeyViolation);
    expect(
      expectOk(await db.fetchScalar<int>('SELECT COUNT(*) FROM children', [])),
      0,
    );
    expectOk(
      await db.transaction<Unit>((tx) async {
        expectOk(await tx.execute('INSERT INTO parents VALUES (1)', []));
        return (await tx.execute(
          'INSERT INTO children VALUES (1)',
          [],
        )).map((_) => unit);
      }),
    );
    expect(
      expectOk(await db.fetchScalar<int>('SELECT COUNT(*) FROM children', [])),
      1,
    );
  });
}
