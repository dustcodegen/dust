import 'dart:isolate';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:dust_db_sqlite3/src/sqlite_pool.dart'
    show initializeSqlite3Connection;
import 'package:sqlite3/sqlite3.dart' as sqlite;
import 'package:test/test.dart';

import 'support/expect_ok.dart';
import 'support/migration_fixture.dart';

void main() {
  test(
    'a competing writer fails at BEGIN before history or pending SQL',
    () async {
      final fixture = MigrationFixture();
      addTearDown(fixture.dispose);
      expectOk(await fixture.open(migrations: firstMigration).close());
      final writer = sqlite.sqlite3.open(fixture.path)
        ..execute('BEGIN IMMEDIATE');
      try {
        expect(
          () => Sqlite3Driver.open(
            fixture.path,
            migrations: upgradedMigrations,
            options: const SqliteConnectOptions(busyTimeout: Duration.zero),
          ),
          throwsA(
            isA<SqlxError>()
                .having(
                  (e) => e.category,
                  'category',
                  SqlxErrorCategory.migration,
                )
                .having((e) => e.operation, 'operation', 'BEGIN IMMEDIATE')
                .having(
                  (e) => (e.cause! as sqlite.SqliteException).resultCode,
                  'native code',
                  5,
                ),
          ),
        );
        expect(writer.autocommit, isFalse);
        expect(
          writer.select('SELECT name FROM __dust_schema_migrations').length,
          1,
        );
      } finally {
        writer.execute('ROLLBACK');
        writer.close();
      }
      expectOk(await fixture.open(migrations: upgradedMigrations).close());
      expect(fixture.read('PRAGMA table_info(items)').map((row) => row[1]), [
        'id',
        'name',
        'active',
      ]);
    },
  );

  test(
    'busy-timeout open waits, then verifies the previous writer commit',
    () async {
      final fixture = MigrationFixture();
      addTearDown(fixture.dispose);
      expectOk(await fixture.open(migrations: firstMigration).close());
      final ready = ReceivePort();
      final send = ready.sendPort;
      final path = fixture.path;
      final writer = Isolate.run(() => _holdWriter(send, path));
      SendPort? release;
      Sqlite3Driver? db;
      try {
        release = await Future.any([ready.first, writer]) as SendPort;
        final native = _ReleaseWriter(sqlite.sqlite3.open(path), release);
        db = initializeSqlite3Connection(
          native,
          SqliteConnectOptions.path(
            path,
            busyTimeout: const Duration(seconds: 2),
          ),
          upgradedMigrations,
        );
        expect(
          expectOk(await db.fetchScalar<int>('SELECT COUNT(*) FROM items', [])),
          1,
        );
        expect(
          native.inner
              .select('SELECT name FROM __dust_schema_migrations ORDER BY name')
              .map((row) => row['name']),
          ['0001.sql', '0002.sql'],
        );
      } finally {
        // Also unblock the writer when initialization/assertion fails.
        release?.send(null);
        if (db != null) expectOk(await db.close());
        await writer;
        ready.close();
      }
    },
  );
}

Future<Object?> _holdWriter(SendPort ready, String path) async {
  final native = sqlite.sqlite3.open(path);
  final release = ReceivePort();
  try {
    native.execute('BEGIN IMMEDIATE');
    native.execute(upgradeItems);
    native.execute("INSERT INTO items (id, name) VALUES (7, 'writer');");
    native.execute(
      "INSERT INTO __dust_schema_migrations (name, applied_at, checksum, checksum_source) VALUES (?, ?, ?, 'applied')",
      ['0002.sql', legacyTimestamp, upgradeChecksum],
    );
    ready.send(release.sendPort);
    await release.first;
    native.execute('COMMIT');
  } finally {
    release.close();
    native.close();
  }
  return null;
}

/// Releases a real isolate-owned writer exactly when startup attempts its lock.
final class _ReleaseWriter implements sqlite.Database {
  _ReleaseWriter(this.inner, this.release);
  final sqlite.Database inner;
  final SendPort release;

  @override
  bool get autocommit => inner.autocommit;

  @override
  sqlite.VoidPredicate? get commitFilter => inner.commitFilter;

  @override
  set commitFilter(sqlite.VoidPredicate? value) => inner.commitFilter = value;

  @override
  void execute(String sql, [List<Object?> parameters = const []]) {
    if (sql == 'BEGIN IMMEDIATE') release.send(null);
    inner.execute(sql, parameters);
  }

  @override
  sqlite.ResultSet select(String sql, [List<Object?> parameters = const []]) =>
      inner.select(sql, parameters);

  @override
  sqlite.PreparedStatement prepare(
    String sql, {
    bool persistent = false,
    bool vtab = true,
    bool checkNoTail = false,
  }) => inner.prepare(
    sql,
    persistent: persistent,
    vtab: vtab,
    checkNoTail: checkNoTail,
  );

  @override
  void close() => inner.close();

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'Unexpected native operation: ${invocation.memberName}',
  );
}
