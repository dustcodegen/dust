import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

/// A deferred foreign key fails at commit, not at insert.
///
/// Run with `dart run example/commit_recovery.dart`.
Future<void> main() async {
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

  try {
    final failed = await db.transaction<Unit>((tx) async {
      return (await tx.execute(
        'INSERT INTO children VALUES (1)',
        const [],
      )).map((_) => unit);
    });
    print(
      failed.match(
        ok: (_) => 'unexpected success',
        err: (error) => 'failed at: ${error.operation}',
      ),
    );
    final count = await db.fetchScalar<int>(
      'SELECT COUNT(*) FROM children',
      const [],
    );
    print(
      'children after rollback: ${count.unwrapOrElse((error) => throw error)}',
    );

    final recovered = await db.transaction<Unit>((tx) async {
      final parent = await tx.execute(
        'INSERT INTO parents VALUES (1)',
        const [],
      );
      if (parent.isErr) return parent.map((_) => unit);
      return (await tx.execute(
        'INSERT INTO children VALUES (1)',
        const [],
      )).map((_) => unit);
    });
    print('next transaction committed: ${recovered.isOk}');
  } finally {
    await db.close();
  }
}
