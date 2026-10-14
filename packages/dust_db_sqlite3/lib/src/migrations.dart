part of 'sqlite_pool.dart';

const _schemaMigrationsTable = '__dust_schema_migrations';

void _applyMigrations(
  sqlite.Database database,
  Map<String, String>? migrations,
  bool baselineLegacy,
) {
  if (migrations == null) return;
  final pending =
      migrations.entries
          .where((migration) => !migration.key.endsWith('.down.sql'))
          .toList()
        ..sort((left, right) => left.key.compareTo(right.key));

  try {
    // Take the writer lock before reading history so concurrent opens serialize.
    database.execute('BEGIN IMMEDIATE');
  } catch (error) {
    throw _sqliteMigrationError(
      'SQLite migration transaction begin failed.',
      cause: error,
      operation: 'BEGIN IMMEDIATE',
    );
  }
  final originalFilter = database.commitFilter;
  var completing = false;
  // Reject both early COMMIT and implicit writes after a migration's ROLLBACK.
  database.commitFilter = () => completing && (originalFilter?.call() ?? true);
  try {
    database.execute('SAVEPOINT _dust_migration_boundary');
    _ensureMigrationTable(database);
    final applied = _appliedMigrations(database);
    final checksums = {
      for (final migration in pending)
        migration.key: sha256.convert(utf8.encode(migration.value)).toString(),
    };
    _verifyMigrationHistory(database, applied, checksums, baselineLegacy);
    for (final migration in pending) {
      if (applied.containsKey(migration.key)) continue;
      _applyMigration(database, migration.key, migration.value);
      _recordMigration(database, migration.key, checksums[migration.key]!);
    }
    // A migration's ROLLBACK + BEGIN must not replace the owned transaction.
    database.execute('RELEASE SAVEPOINT _dust_migration_boundary');
    completing = true;
    database.execute('COMMIT');
  } catch (error) {
    final migrationError = _asMigrationError(error);
    try {
      if (!database.autocommit) database.execute('ROLLBACK');
    } catch (rollbackError) {
      throw _sqliteMigrationError(
        'SQLite migration rollback failed.',
        cause: rollbackError,
        operation: 'ROLLBACK',
      );
    }
    throw migrationError;
  } finally {
    database.commitFilter = originalFilter;
  }
}

void _ensureMigrationTable(sqlite.Database database) {
  try {
    database.execute('''
CREATE TABLE IF NOT EXISTS $_schemaMigrationsTable (
  name TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  checksum TEXT,
  checksum_source TEXT NOT NULL DEFAULT 'legacy-baseline'
)
''');
    final columns = database
        .select('PRAGMA table_info($_schemaMigrationsTable)')
        .map((row) => row['name'])
        .toSet();
    if (!columns.contains('checksum')) {
      database.execute(
        'ALTER TABLE $_schemaMigrationsTable ADD COLUMN checksum TEXT',
      );
    }
    if (!columns.contains('checksum_source')) {
      database.execute(
        "ALTER TABLE $_schemaMigrationsTable ADD COLUMN checksum_source "
        "TEXT NOT NULL DEFAULT 'legacy-baseline'",
      );
    }
  } catch (error) {
    throw _sqliteMigrationError(
      'SQLite migration table setup failed.',
      cause: error,
      operation: _schemaMigrationsTable,
    );
  }
}

void _applyMigration(sqlite.Database database, String name, String sql) {
  try {
    database.execute(sql);
  } catch (error) {
    throw _sqliteMigrationError(
      'SQLite migration `$name` failed.',
      cause: error,
      operation: name,
    );
  }
}

void _recordMigration(sqlite.Database database, String name, String checksum) {
  late final sqlite.PreparedStatement statement;
  try {
    statement = database.prepare(
      'INSERT INTO $_schemaMigrationsTable '
      '(name, applied_at, checksum, checksum_source) '
      "VALUES (?, ?, ?, 'applied')",
    );
  } catch (error) {
    throw _sqliteMigrationError(
      'SQLite migration history prepare failed.',
      cause: error,
      operation: name,
    );
  }
  try {
    statement.execute([
      name,
      DateTime.now().toUtc().toIso8601String(),
      checksum,
    ]);
  } catch (error) {
    throw _sqliteMigrationError(
      'SQLite migration history write failed.',
      cause: error,
      operation: name,
    );
  } finally {
    statement.close();
  }
}

SqlxError _asMigrationError(Object error) {
  if (error is SqlxError) return error;
  return _sqliteMigrationError(
    'SQLite migration failed.',
    cause: error,
  );
}
