import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

/// A failed worker COMMIT rolls back before the next transaction starts.
///
/// `dart run example/background_commit_recovery.dart`
Future<void> main() async {
  final db = await Sqlite3WorkerDriver.connect(
    const SqliteConnectOptions.memory(foreignKeys: true),
    migrations: const {
      '0001.sql': '''
CREATE TABLE parents (id INTEGER PRIMARY KEY);
CREATE TABLE children (
  id INTEGER REFERENCES parents(id) DEFERRABLE INITIALLY DEFERRED
);
''',
    },
  );
  try {
    final failed = await db.transaction<Unit>((tx) async {
      return (await tx.execute(
        'INSERT INTO children VALUES (1)',
        [],
      )).map((_) => unit);
    });
    print('failed at: ${failed.unwrapErr().operation}');
    final remaining = (await db.fetchScalar<int>(
      'SELECT COUNT(*) FROM children',
      [],
    )).unwrap();
    print('children after rollback: $remaining');
    final next = await db.transaction<Unit>((tx) async {
      final parent = await tx.execute('INSERT INTO parents VALUES (1)', []);
      if (parent case Err(:final error)) return Err(error);
      return (await tx.execute(
        'INSERT INTO children VALUES (1)',
        [],
      )).map((_) => unit);
    });
    print('next transaction committed: ${next.isOk}');
  } finally {
    (await db.close()).unwrap();
  }
}
