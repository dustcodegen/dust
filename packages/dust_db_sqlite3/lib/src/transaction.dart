part of 'sqlite_pool.dart';

/// State belonging to one open database, not to one driver over it.
///
/// A transaction is a second `Sqlite3Driver` over the same `sqlite.Database`,
/// so anything scoped to the connection has to be shared with it rather than
/// rebuilt: savepoint names have to stay unique across nesting, and a prepared
/// statement is worth reusing whether the caller holds the pool or a
/// transaction on it.
final class _ConnectionState {
  var _nextSavepointId = 0;
  var transactionActive = false;
  SqlxError? recoveryError;

  SqlxError _failedCleanup(SqlxError error) {
    recoveryError ??= _sqliteTransactionError(
      'SQLite transaction cleanup failed. Close and reopen this connection.',
      cause: error,
      operation: error.operation,
    );
    return error;
  }

  SqlxError? _rollbackRoot(sqlite.Database database, String message) {
    final error = _control(database, 'ROLLBACK', message);
    return error == null ? null : _failedCleanup(error);
  }

  /// Prepared statements held for this connection.
  final _StatementCache statements = _StatementCache();

  /// Runs one control statement whose text never varies.
  ///
  /// `BEGIN`, `COMMIT` and `ROLLBACK` are the same three strings for the life
  /// of the process, so they are held like any other statement — a transaction
  /// per request would otherwise compile two of them every time, which
  /// measured about 0.7us of a 3.6us transaction.
  ///
  /// Savepoints do not come here: their names carry a counter, so every one is
  /// a statement seen once, and caching them would fill the cache with entries
  /// that can never hit.
  SqlxError? _control(sqlite.Database database, String sql, String message) {
    try {
      statements.statementFor(database, sql).execute();
      return null;
    } catch (error) {
      return _sqliteTransactionError(message, cause: error, operation: sql);
    }
  }

  Future<Result<T, SqlxError>> runRoot<T>(
    sqlite.Database database,
    Future<Result<T, SqlxError>> Function(Transaction tx) fn,
  ) async {
    final begin = _control(
      database,
      'BEGIN',
      'SQLite transaction begin failed.',
    );
    if (begin != null) return Err<T, SqlxError>(begin);

    final tx = _SingleConnectionPool(database, this);
    transactionActive = true;
    try {
      final result = await tx._invoke(fn);
      return await result.match<Future<Result<T, SqlxError>>>(
        ok: (value) async {
          final recovery = recoveryError;
          if (recovery != null) {
            final rollback = _rollbackRoot(
              database,
              'SQLite transaction rollback failed.',
            );
            return Err<T, SqlxError>(rollback ?? recovery);
          }
          final commit = _control(
            database,
            'COMMIT',
            'SQLite transaction commit failed.',
          );
          if (commit != null) {
            // A failed COMMIT can leave uncommitted writes for the next caller.
            final rollback = database.autocommit
                ? null
                : _rollbackRoot(
                    database,
                    'SQLite rollback after failed commit failed.',
                  );
            return Err<T, SqlxError>(rollback ?? commit);
          }
          return Ok<T, SqlxError>(value);
        },
        err: (error) async {
          final rollback = _rollbackRoot(
            database,
            'SQLite transaction rollback failed.',
          );
          if (rollback != null) return Err<T, SqlxError>(rollback);
          return Err<T, SqlxError>(error);
        },
      );
    } catch (error) {
      final rollback = _rollbackRoot(
        database,
        'SQLite transaction rollback failed.',
      );
      if (rollback != null) return Err<T, SqlxError>(rollback);
      return Err<T, SqlxError>(
        _sqliteTransactionError(
          'SQLite transaction failed.',
          cause: error,
          operation: 'transaction callback',
        ),
      );
    } finally {
      tx._deactivate();
      transactionActive = false;
    }
  }

  Future<Result<T, SqlxError>> runSavepoint<T>(
    sqlite.Database database,
    Future<Result<T, SqlxError>> Function(Transaction tx) fn,
  ) async {
    final name = '_dust_tx_${++_nextSavepointId}';
    final begin = _executeControl(
      database,
      'SAVEPOINT $name',
      'SQLite savepoint begin failed.',
    );
    if (begin != null) return Err<T, SqlxError>(begin);

    final tx = _SingleConnectionPool(database, this);
    try {
      final result = await tx._invoke(fn);
      return await result.match<Future<Result<T, SqlxError>>>(
        ok: (value) async {
          final release = _executeControl(
            database,
            'RELEASE SAVEPOINT $name',
            'SQLite savepoint release failed.',
          );
          if (release != null) {
            return Err<T, SqlxError>(_failedCleanup(release));
          }
          return Ok<T, SqlxError>(value);
        },
        err: (error) async {
          final rollback = _rollbackSavepoint(database, name);
          if (rollback != null) return Err<T, SqlxError>(rollback);
          return Err<T, SqlxError>(error);
        },
      );
    } catch (error) {
      final rollback = _rollbackSavepoint(database, name);
      if (rollback != null) return Err<T, SqlxError>(rollback);
      return Err<T, SqlxError>(
        _sqliteTransactionError(
          'SQLite transaction failed.',
          cause: error,
          operation: name,
        ),
      );
    } finally {
      tx._deactivate();
    }
  }

  SqlxError? _rollbackSavepoint(sqlite.Database database, String name) {
    final rollback = _executeControl(
      database,
      'ROLLBACK TO SAVEPOINT $name',
      'SQLite savepoint rollback failed.',
    );
    final release = _executeControl(
      database,
      'RELEASE SAVEPOINT $name',
      'SQLite savepoint release after rollback failed.',
    );
    final error = rollback ?? release;
    return error == null ? null : _failedCleanup(error);
  }
}

extension _Sqlite3TransactionRunner on Sqlite3Driver {
  Future<Result<T, SqlxError>> _runTransaction<T>(
    Future<Result<T, SqlxError>> Function(Transaction tx) fn,
  ) {
    final error = _closedError();
    if (error != null) return Future.value(Err<T, SqlxError>(error));
    if (!_ownsDatabase) return _connection.runSavepoint(database, fn);
    return _connection.runRoot(database, fn);
  }
}

/// Runs one control statement whose text is written for this call.
///
/// Savepoint statements name a counter, so each is seen once. Compiling and
/// discarding is the right thing for a statement that cannot be reused; see
/// `_ConnectionState._control` for the ones that can.
SqlxError? _executeControl(
  sqlite.Database database,
  String sql,
  String message,
) {
  try {
    database.execute(sql);
    return null;
  } catch (error) {
    return _sqliteTransactionError(
      message,
      cause: error,
      operation: sql,
    );
  }
}
