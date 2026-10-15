import 'dart:isolate';
import 'dart:typed_data';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  late Sqlite3WorkerDriver db;
  setUp(
    () async => db = await Sqlite3WorkerDriver.open(
      ':memory:',
      migrations: const {
        '0001.sql':
            'CREATE TABLE notes(id INTEGER PRIMARY KEY, title TEXT UNIQUE);',
      },
    ),
  );
  tearDown(() async => expectOk(await db.close()));

  test('all row terminals keep unsendable mappers on the caller', () async {
    final caller = Isolate.current;
    final port = ReceivePort();
    addTearDown(port.close);
    final inserted = expectOk(
      await db.execute(r'INSERT INTO notes(title) VALUES ($1)', ['Ada 🍜']),
    );
    expect(inserted.rowsAffected, 1);
    expect(inserted.lastInsertId, 1);
    String mapper(Row row) {
      expect(Isolate.current, same(caller));
      expect(port.sendPort, isA<SendPort>());
      return row.read<String>('title');
    }

    expect(
      expectOk(await db.fetchOne('SELECT title FROM notes', [], mapper)),
      'Ada 🍜',
    );
    expect(
      expectOk(await db.fetchOptional('SELECT title FROM notes', [], mapper)),
      'Ada 🍜',
    );
    expect(expectOk(await db.fetchAll('SELECT title FROM notes', [], mapper)), [
      'Ada 🍜',
    ]);
    expect(
      expectOk(
        await db.fetchOptional('SELECT title FROM notes WHERE 0', [], mapper),
      ),
      isNull,
    );
    expect(
      expectOk(
        await db.fetchAll('SELECT title FROM notes WHERE 0', [], mapper),
      ),
      isEmpty,
    );
    expect(
      (await db.fetchOne(
        'SELECT title FROM notes WHERE 0',
        [],
        mapper,
      )).unwrapErr(),
      isA<SqlxCardinalityError>(),
    );
    expect(
      (await db.fetchOne(
        'SELECT 1 UNION ALL SELECT 2',
        [],
        mapper,
      )).unwrapErr(),
      isA<SqlxCardinalityError>(),
    );
    final error = (await db.execute('INSERT INTO notes(title) VALUES (?)', [
      'Ada 🍜',
    ])).unwrapErr();
    expect(error.kind, SqlxErrorKind.uniqueViolation);
    expect(error.driver, Driver.sqlite3);
    expect(error.category, SqlxErrorCategory.query);
    expect(error.cause, isA<String>());
  });

  test(
    'scalar cardinality, nulls and decode failures match the native driver',
    () async {
      expect(expectOk(await db.fetchScalar<int>('SELECT 1', [])), 1);
      expect(expectOk(await db.fetchScalar<double>('SELECT 1', [])), 1.0);
      expect(expectOk(await db.fetchScalar<int?>('SELECT 1', [])), 1);
      expect(expectOk(await db.fetchScalar<int?>('SELECT NULL', [])), isNull);
      expect(
        expectOk(await db.fetchScalar<int?>('SELECT 1 WHERE 0', [])),
        isNull,
      );
      expect(
        (await db.fetchScalar<int>('SELECT 1 WHERE 0', [])).unwrapErr(),
        isA<SqlxCardinalityError>(),
      );
      expect(
        (await db.fetchScalar<int>(
          'SELECT 1 UNION ALL SELECT 2',
          [],
        )).unwrapErr(),
        isA<SqlxCardinalityError>(),
      );
      expect(
        (await db.fetchScalar<int>("SELECT 'bad'", [])).unwrapErr().category,
        SqlxErrorCategory.decode,
      );
      expect(
        (await db.fetchScalar<int>('SELECT NULL', [])).unwrapErr().category,
        SqlxErrorCategory.decode,
      );
      expect(
        expectOk(
          await db.fetchOne(
            '''SELECT 1 AS "id!", 1 AS enabled, '2026-01-01T03:30:00Z' AS created''',
            [],
            (row) => (
              row.read<int>('id'),
              row.readBool('enabled'),
              row.readDateTime('created'),
            ),
          ),
        ),
        (1, true, DateTime.utc(2026, 1, 1, 3, 30)),
      );
    },
  );

  test(
    'list, BLOB and scalar inputs are captured before caller mutation',
    () async {
      expectOk(
        await db.execute(
          'CREATE TABLE payloads (id INTEGER, bytes BLOB, ids TEXT)',
          [],
        ),
      );
      final bytes = Uint8List.fromList([1, 2]);
      final ids = [1, 3];
      final parameters = <Object?>[7, bytes, ids];
      final inserted = db.execute(
        r'INSERT INTO payloads VALUES ($1, $2, $3)',
        parameters,
      );
      parameters[0] = 9;
      bytes[0] = 9;
      ids[0] = 9;
      expectOk(await inserted);
      expect(
        expectOk(await db.fetchScalar<int>('SELECT id FROM payloads', [])),
        7,
      );
      expect(
        expectOk(
          await db.fetchScalar<Uint8List>('SELECT bytes FROM payloads', []),
        ),
        [1, 2],
      );
      expect(
        expectOk(await db.fetchScalar<String>('SELECT ids FROM payloads', [])),
        '[1,3]',
      );
      expect(
        expectOk(
          await db.fetchScalar<int>(r'SELECT SUM(value) FROM json_each($1)', [
            [1, 3],
          ]),
        ),
        4,
      );
    },
  );

  test(
    'invalid binds and mapper errors do not terminate a healthy worker',
    () async {
      final port = ReceivePort();
      addTearDown(port.close);
      for (final parameters in [
        [port],
        [const Object()],
        [
          [const Object()],
        ],
      ]) {
        expect(
          (await db.fetchScalar<int>(
            r'SELECT $1',
            parameters,
          )).unwrapErr().category,
          SqlxErrorCategory.query,
        );
      }
      expect(
        (await db.fetchScalar<int>(r'SELECT $2', [1])).unwrapErr().category,
        SqlxErrorCategory.query,
      );
      final failure = SqlxError.decode('caller mapper');
      expect(
        (await db.fetchOne<int>(
          'SELECT 1',
          [],
          (_) => throw failure,
        )).unwrapErr(),
        same(failure),
      );
      expect(
        (await db.fetchOne<int>(
          'SELECT 1',
          [],
          (_) => throw StateError('mapper'),
        )).unwrapErr().category,
        SqlxErrorCategory.decode,
      );
      expect(
        (await db.fetchOptional<int>(
          'SELECT 1',
          [],
          (_) => throw failure,
        )).unwrapErr(),
        same(failure),
      );
      var calls = 0;
      expect(
        (await db.fetchAll<int>(
          'SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3',
          [],
          (row) {
            calls++;
            if (calls == 2) throw failure;
            return row.readIndex<int>(0);
          },
        )).unwrapErr(),
        same(failure),
      );
      expect(calls, 2);
      expect(expectOk(await db.fetchScalar<int>('SELECT 9', [])), 9);
    },
  );
}
