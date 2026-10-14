part of 'sqlite_pool.dart';

typedef _MigrationRecord = ({String? checksum, String source});

Map<String, _MigrationRecord> _appliedMigrations(sqlite.Database database) {
  try {
    final rows = database.select(
      'SELECT name, checksum, checksum_source '
      'FROM $_schemaMigrationsTable ORDER BY name',
    );
    final applied = <String, _MigrationRecord>{};
    for (final row in rows) {
      final name = row['name'] as String;
      if (applied.containsKey(name)) {
        throw StateError('Duplicate migration history for `$name`.');
      }
      applied[name] = (
        checksum: row['checksum'] as String?,
        source: row['checksum_source'] as String,
      );
    }
    return applied;
  } catch (error) {
    throw _sqliteMigrationError(
      'SQLite migration history read failed.',
      cause: error,
      operation: _schemaMigrationsTable,
    );
  }
}

void _verifyMigrationHistory(
  sqlite.Database database,
  Map<String, _MigrationRecord> applied,
  Map<String, String> checksums,
  bool baselineLegacy,
) {
  for (final entry in applied.entries) {
    final name = entry.key;
    final expected = checksums[name];
    if (expected == null) {
      throw _sqliteMigrationError(
        'SQLite migration `$name` is missing from the supplied bundle.',
        operation: name,
      );
    }
    final recorded = entry.value;
    if (recorded.source != 'applied' && recorded.source != 'legacy-baseline') {
      throw _sqliteMigrationError(
        'SQLite migration `$name` checksum provenance is invalid.',
        operation: name,
      );
    }
    if (recorded.checksum == null) {
      if (recorded.source != 'legacy-baseline') {
        throw _sqliteMigrationError(
          'SQLite migration `$name` checksum is missing.',
          operation: name,
        );
      }
      if (!baselineLegacy) {
        throw _sqliteMigrationError(
          'SQLite migration `$name` has unverified legacy history. '
          'Review the deployed SQL and explicitly enable baselineLegacyMigrations.',
          operation: name,
        );
      }
      _baselineMigration(database, name, expected);
    } else if (recorded.checksum != expected) {
      throw _sqliteMigrationError(
        'SQLite migration `$name` checksum mismatch.',
        operation: name,
      );
    }
  }
}

void _baselineMigration(
  sqlite.Database database,
  String name,
  String checksum,
) {
  try {
    database.execute(
      'UPDATE $_schemaMigrationsTable SET checksum = ?, '
      "checksum_source = 'legacy-baseline' WHERE name = ?",
      [checksum, name],
    );
  } catch (error) {
    throw _sqliteMigrationError(
      'SQLite migration checksum baseline failed.',
      cause: error,
      operation: name,
    );
  }
}
