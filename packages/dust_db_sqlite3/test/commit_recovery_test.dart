import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

import 'support/expect_ok.dart';

void main() {
  group('failed commit recovery', () {
    test('deferred constraint rolls back writes and permits reuse', () async {
      final db = Sqlite3Driver.connect(
        const SqliteConnectOptions.memory(foreignKeys: true),
        migrations: const {
          '0001.sql': '''
CREATE TABLE parents (id INTEGER PRIMARY KEY);
CREATE TABLE children (
  parent_id INTEGER REFERENCES parents(id) DEFERRABLE INITIALLY DEFERRED
);
''',
        },
      );
      addTearDown(db.close);
      late Transaction captured;

      final result = await db.transaction<Unit>((tx) async {
        captured = tx;
        expectOk(await tx.execute('INSERT INTO parents VALUES (2)', const []));
        expectOk(await tx.execute('INSERT INTO children VALUES (1)', const []));
        return const Ok<Unit, SqlxError>(unit);
      });

      final error = _expectCommitError(result);
      expect(error.kind, SqlxErrorKind.foreignKeyViolation);
      expect(db.database.autocommit, isTrue);
      expect(
        expectOk(
          await db.fetchScalar<int>(
            'SELECT COUNT(*) FROM parents',
            const [],
          ),
        ),
        0,
      );
      expect(
        expectOk(
          await db.fetchScalar<int>(
            'SELECT COUNT(*) FROM children',
            const [],
          ),
        ),
        0,
      );
      expect(
        await captured.execute('INSERT INTO parents VALUES (1)', const []),
        isA<Err<ExecResult, SqlxError>>(),
      );
      expect(
        await captured.transaction<Unit>(
          (_) async => const Ok<Unit, SqlxError>(unit),
        ),
        isA<Err<Unit, SqlxError>>(),
      );

      expectOk(
        await db.transaction<Unit>((tx) async {
          expectOk(
            await tx.execute('INSERT INTO parents VALUES (1)', const []),
          );
          return (await tx.execute(
            'INSERT INTO children VALUES (1)',
            const [],
          )).map((_) => unit);
        }),
      );
      expect(
        expectOk(
          await db.fetchScalar<int>(
            'SELECT COUNT(*) FROM children',
            const [],
          ),
        ),
        1,
      );
    });

    test('busy commit rolls back before returning and permits reuse', () async {
      final directory = Directory.systemTemp.createTempSync('dust_commit_');
      addTearDown(() => directory.deleteSync(recursive: true));
      final path = '${directory.path}/app.db';
      final writer = Sqlite3Driver.open(
        path,
        options: const SqliteConnectOptions(
          journalMode: SqliteJournalMode.delete,
          busyTimeout: Duration.zero,
        ),
        migrations: const {
          '0001.sql': 'CREATE TABLE users (id INTEGER PRIMARY KEY);',
        },
      );
      addTearDown(writer.close);
      final reader = Sqlite3Driver.open(path);
      addTearDown(reader.close);
      reader.database.execute('BEGIN');
      reader.database.select('SELECT * FROM users');

      final result = await writer.transaction<Unit>((tx) async {
        return (await tx.execute(
          'INSERT INTO users VALUES (1)',
          const [],
        )).map((_) => unit);
      });

      final error = _expectCommitError(result);
      expect(
        error.cause,
        isA<sqlite.SqliteException>().having(
          (cause) => cause.resultCode,
          'SQLITE_BUSY',
          5,
        ),
      );
      expect(writer.database.autocommit, isTrue);
      reader.database.execute('ROLLBACK');
      expect(
        expectOk(
          await writer.fetchScalar<int>(
            'SELECT COUNT(*) FROM users',
            const [],
          ),
        ),
        0,
      );
      expectOk(
        await writer.transaction<Unit>((tx) async {
          return (await tx.execute(
            'INSERT INTO users VALUES (2)',
            const [],
          )).map((_) => unit);
        }),
      );
      expect(
        expectOk(
          await reader.fetchScalar<int>('SELECT id FROM users', const []),
        ),
        2,
      );
    });

    test(
      'already-ended transaction retains the original commit error',
      () async {
        final db = Sqlite3Driver.open(':memory:');
        addTearDown(db.close);

        final result = await db.transaction<Unit>((tx) async {
          db.database.execute('ROLLBACK');
          return const Ok<Unit, SqlxError>(unit);
        });

        _expectCommitError(result);
        expect(db.database.autocommit, isTrue);
        expectOk(
          await db.transaction<Unit>(
            (_) async => const Ok<Unit, SqlxError>(unit),
          ),
        );
      },
    );

    test(
      'cleanup failure is reported instead of a reusable connection',
      () async {
        final db = Sqlite3Driver.open(':memory:');
        addTearDown(db.close);

        final result = await db.transaction<Unit>((tx) async {
          db.database.close();
          return const Ok<Unit, SqlxError>(unit);
        });

        final error = result.match(
          ok: (_) => fail('expected rollback failure'),
          err: (error) => error,
        );
        expect(error.category, SqlxErrorCategory.transaction);
        expect(error.driver, Driver.sqlite3);
        expect(error.operation, 'ROLLBACK');
        expect(error.cause, isA<StateError>());
        expect(
          await db.fetchScalar<int>('SELECT 1', const []),
          isA<Err<int, SqlxError>>(),
        );
      },
    );
  });
}

SqlxError _expectCommitError(Result<Unit, SqlxError> result) {
  expect(result, isA<Err<Unit, SqlxError>>());
  final error = result.match(
    ok: (_) => fail('expected commit failure'),
    err: (error) => error,
  );
  expect(error.category, SqlxErrorCategory.transaction);
  expect(error.driver, Driver.sqlite3);
  expect(error.operation, 'COMMIT');
  return error;
}
