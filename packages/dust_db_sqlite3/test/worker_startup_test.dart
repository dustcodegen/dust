import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  test('worker is ready only after configuration and migrations', () async {
    final db = await Sqlite3WorkerDriver.connect(
      const SqliteConnectOptions.memory(foreignKeys: true),
      migrations: const {
        '0001.sql':
            'CREATE TABLE items (id INTEGER PRIMARY KEY); INSERT INTO items VALUES (1);',
      },
    );
    addTearDown(db.close);
    expect(db.driver, Driver.sqlite3);
    expect(db, isNot(isA<Sqlite3Executor>()));
    expect(expectOk(await db.fetchScalar<int>('PRAGMA foreign_keys', [])), 1);
    expect(
      expectOk(await db.fetchScalar<int>('SELECT COUNT(*) FROM items', [])),
      1,
    );
    expectOk(await db.close());
    expectOk(await db.close());
  });

  test(
    'file-backed worker commits data that survives close and reopen',
    () async {
      final directory = Directory.systemTemp.createTempSync('dust-worker-');
      addTearDown(() => directory.deleteSync(recursive: true));
      final path = '${directory.path}/app.db';
      const migrations = {'0001.sql': 'CREATE TABLE items (id INTEGER);'};
      final first = await Sqlite3WorkerDriver.open(
        path,
        migrations: migrations,
      );
      try {
        expectOk(await first.execute('INSERT INTO items VALUES (?)', [7]));
      } finally {
        expectOk(await first.close());
      }
      final reopened = await Sqlite3WorkerDriver.open(
        path,
        migrations: migrations,
      );
      try {
        expect(
          expectOk(await reopened.fetchScalar<int>('SELECT id FROM items', [])),
          7,
        );
      } finally {
        expectOk(await reopened.close());
      }
    },
  );

  test(
    'invalid options and migration failure reject startup with typed errors',
    () async {
      await expectLater(
        Sqlite3WorkerDriver.connect(const SqliteConnectOptions()),
        throwsA(isA<SqlxError>()),
      );
      await expectLater(
        Sqlite3WorkerDriver.open(
          ':memory:',
          migrations: {'0001.sql': 'INVALID SQL;'},
        ),
        throwsA(
          isA<SqlxError>()
              .having(
                (e) => e.category,
                'category',
                SqlxErrorCategory.migration,
              )
              .having((e) => e.operation, 'operation', '0001.sql'),
        ),
      );
    },
  );
}
