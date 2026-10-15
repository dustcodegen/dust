part of 'sqlite_pool.dart';

/// One native SQLite connection owned by a persistent background isolate.
///
/// Opening, migrations, SQL and native close run in the worker. Row mapping
/// and transaction callbacks run on the caller. No native handle is exposed.
final class Sqlite3WorkerDriver extends _WorkerExecutor implements Pool {
  Sqlite3WorkerDriver._(_WorkerTransport transport)
    : super(_WorkerConnection(transport));

  /// Opens a worker, resolving only after options and migrations succeed.
  ///
  /// Omitted/null [migrations] mean unmanaged startup. An explicitly empty
  /// bundle still verifies history. Startup failures throw [SqlxError].
  static Future<Sqlite3WorkerDriver> open(
    String path, {
    Map<String, String>? migrations,
    SqliteConnectOptions? options,
  }) async => connect(
    (options ?? const SqliteConnectOptions())._withPath(path),
    migrations: migrations,
  );

  /// Opens a worker using explicit per-connection options.
  static Future<Sqlite3WorkerDriver> connect(
    SqliteConnectOptions options, {
    Map<String, String>? migrations,
  }) async => openSqlite3WorkerConnection(options, migrations);

  @override
  Future<Result<Unit, SqlxError>> close() {
    final closed = _connection.closed;
    if (closed != null) return closed;
    return _access('close', () {
      _connection.closing = true;
      return _connection.closed = _close();
    });
  }

  Future<Result<Unit, SqlxError>> _close() async {
    final result = await _connection.transport.request(['close', null]);
    _connection.transport.dispose();
    return result.map((_) => unit);
  }
}

/// Package-internal startup seam, hidden by the package entrypoint.
///
/// Test entrypoints create native resources inside their worker, never on the
/// caller. There is no public worker hook or native-handle transfer API.
Future<Sqlite3WorkerDriver> openSqlite3WorkerConnection(
  SqliteConnectOptions options,
  Map<String, String>? migrations, {
  void Function(List<Object?>) entrypoint = _sqliteWorkerMain,
  void Function(SendPort, Object?) send = _sendWorkerMessage,
}) async {
  options._validate(hasMigrations: migrations != null);
  final transport = _WorkerTransport(send);
  await transport.start(
    options,
    migrations == null ? null : Map<String, String>.of(migrations),
    entrypoint,
  );
  return Sqlite3WorkerDriver._(transport);
}

final class _WorkerConnection {
  _WorkerConnection(this.transport);
  final _WorkerTransport transport;
  var transactionActive = false;
  var closing = false;
  Future<Result<Unit, SqlxError>>? closed;
}

abstract class _WorkerExecutor implements Executor {
  _WorkerExecutor(this._connection, [this._scope, this._txId]);
  final _WorkerConnection _connection;
  final _TransactionScope? _scope;
  final int? _txId;

  @override
  Driver get driver => Driver.sqlite3;

  @override
  Future<Result<T?, SqlxError>> fetchOptional<T>(
    String sql,
    List<Object?> parameters,
    RowMapper<T> mapper,
  ) => _fetchOptional(sql, parameters, mapper);
  @override
  Future<Result<T, SqlxError>> fetchOne<T>(
    String sql,
    List<Object?> parameters,
    RowMapper<T> mapper,
  ) => _fetchOne(sql, parameters, mapper);
  @override
  Future<Result<List<T>, SqlxError>> fetchAll<T>(
    String sql,
    List<Object?> parameters,
    RowMapper<T> mapper,
  ) => _fetchAll(sql, parameters, mapper);
  @override
  Future<Result<T, SqlxError>> fetchScalar<T>(
    String sql,
    List<Object?> parameters,
  ) => _fetchScalar(sql, parameters);
  @override
  Future<Result<ExecResult, SqlxError>> execute(
    String sql,
    List<Object?> parameters,
  ) => _execute(sql, parameters);

  Future<Result<T, SqlxError>> _access<T>(
    String operation,
    Future<Result<T, SqlxError>> Function() action,
  ) {
    if (_connection.closing) {
      return Future.value(
        Err(
          _sqliteConnectionError(
            'SQLite worker connection is closing or closed.',
            operation: operation,
          ),
        ),
      );
    }
    final scope = _scope;
    final current = Zone.current[_connection] as _TransactionScope?;
    String? message;
    if (scope != null && !scope.active) {
      message = 'SQLite transaction is closed.';
    } else if (current != null) {
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
      return Future.value(
        Err(_sqliteTransactionError(message, operation: operation)),
      );
    }
    return action();
  }

  @override
  Future<Result<T, SqlxError>> transaction<T>(
    Future<Result<T, SqlxError>> Function(Transaction tx) fn,
  ) => _access('transaction', () {
    final scope = _scope;
    if (scope != null) return scope.runNested(() => _runWorkerTransaction(fn));
    _connection.transactionActive = true;
    return _runWorkerTransaction(
      fn,
    ).whenComplete(() => _connection.transactionActive = false);
  });
}
