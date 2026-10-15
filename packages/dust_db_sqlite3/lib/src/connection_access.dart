part of 'sqlite_pool.dart';

extension _Sqlite3ConnectionAccess on Sqlite3Driver {
  void _checkOpen() {
    final error = _closedError();
    if (error != null) throw error;
  }

  SqlxError? _closedError({bool closing = false}) {
    if (_closed) {
      return _sqliteConnectionError(
        'SQLite connection is closed.',
        operation: 'checkOpen',
      );
    }
    final scope = _transactionScope;
    if (scope != null && !scope.active) {
      return _sqliteTransactionError(
        'SQLite transaction is closed.',
        operation: 'checkTransactionOpen',
      );
    }
    final current = Zone.current[_connection] as _TransactionScope?;
    String? message;
    if (current != null) {
      if (!current.active) {
        message = 'SQLite transaction scope is closed.';
      } else if (!identical(scope, current)) {
        message =
            'Use the current transaction executor, not a root or parent handle.';
      } else if (current.nestedDone != null) {
        message =
            'Await the active nested transaction before using its parent.';
      }
    } else if (scope != null) {
      message = 'Use a transaction executor only within its callback.';
    } else if (_connection.transactionActive) {
      message =
          'SQLite connection is owned by an active transaction. Retry after it finishes.';
    }
    if (message != null) {
      return _sqliteTransactionError(
        message,
        operation: 'checkTransactionOwner',
      );
    }
    return closing
        ? null
        : _connection.recoveryError ?? _connection.abortedTransactionError;
  }
}
