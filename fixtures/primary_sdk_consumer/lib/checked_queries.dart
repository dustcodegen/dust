import 'package:dust_dart/db.dart';

import 'private_models.dart';

/// Inserts an account without a DAO or executor extension.
Future<Result<ExecResult, SqlxError>> insertAccount(
  Executor db,
  String name,
  String? note,
) => queryExecute(r'INSERT INTO accounts VALUES ($1, $2)', [
  name,
  note,
]).execute(db);

/// Requires exactly one primary-constructor row.
Future<Result<PrivateAccount, SqlxError>> oneAccount(
  Executor db,
  String name,
) => queryAs<PrivateAccount>(
  r'SELECT display_name, note FROM accounts WHERE display_name = $1',
  [name],
).fetchOne(db);

/// Returns null when no row matches.
Future<Result<PrivateAccount?, SqlxError>> optionalAccount(
  Executor db,
  String name,
) => queryAs<PrivateAccount>(
  r'SELECT display_name, note FROM accounts WHERE display_name = $1',
  [name],
).fetchOptional(db);

/// Returns primary-constructor rows in a deterministic order.
Future<Result<List<PrivateAccount>, SqlxError>> allAccounts(Executor db) =>
    queryAs<PrivateAccount>(
      'SELECT display_name, note FROM accounts ORDER BY display_name',
      [],
    ).fetchAll(db);

/// Reads one scalar without a DAO.
Future<Result<int, SqlxError>> countAccounts(Executor db) =>
    queryScalar<int>('SELECT COUNT(*) FROM accounts', []).fetchOne(db);
