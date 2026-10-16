import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

import 'private_models.dart';

part 'database.g.dart';

/// Migrated database for SQLx-checked primary row queries.
@SqlxDatabase(type: SqlxDatabaseType.sqlite, migrations: './migrations')
abstract class PrimaryDatabase implements DatabaseClient {
  /// Opens the owned database on the caller isolate.
  factory PrimaryDatabase.open(String path) = _$PrimaryDatabase.open;

  /// Opens the owned database and runs SQL on its worker isolate.
  static Future<PrimaryDatabase> openBackground(String path) =>
      _$PrimaryDatabase.openBackground(path);

  @override
  Connection get connection;
}

/// The same checked SQL and primary mapper as the direct query functions.
@SqlxDao()
abstract final class PrimaryAccountDao {
  /// Uses either a connection or its active transaction.
  const factory PrimaryAccountDao(Executor db) = _$PrimaryAccountDao;

  /// Inserts an account with a nullable note.
  @Query(r'INSERT INTO accounts VALUES ($1, $2)')
  Future<Result<ExecResult, SqlxError>> insert(String name, String? note);

  /// Requires exactly one account.
  @Query(r'SELECT display_name, note FROM accounts WHERE display_name = $1')
  Future<Result<PrivateAccount, SqlxError>> one(String name);

  /// Returns null when no account matches.
  @Query(r'SELECT display_name, note FROM accounts WHERE display_name = $1')
  Future<Result<PrivateAccount?, SqlxError>> optional(String name);

  /// Reads accounts in a deterministic order.
  @Query('SELECT display_name, note FROM accounts ORDER BY display_name')
  Future<Result<List<PrivateAccount>, SqlxError>> all();

  /// Reads one scalar.
  @Query('SELECT COUNT(*) FROM accounts')
  Future<Result<int, SqlxError>> count();
}
