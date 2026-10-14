part of 'sqlite_pool.dart';

final class _TransactionScope {
  var active = true;
  Future<void>? nestedDone;

  Future<Result<T, SqlxError>> runNested<T>(
    Future<Result<T, SqlxError>> Function() action,
  ) {
    final done = Completer<void>();
    nestedDone = done.future;
    return action().whenComplete(() {
      nestedDone = null;
      done.complete();
    });
  }
}

final class _SingleConnectionPool implements Transaction, Sqlite3Executor {
  _SingleConnectionPool(
    sqlite.Database database,
    _ConnectionState connection,
  ) : this._scoped(database, connection, _TransactionScope());

  _SingleConnectionPool._scoped(
    sqlite.Database database,
    this._connection,
    this._scope,
  ) : _driver = Sqlite3Driver._(
        database,
        ownsDatabase: false,
        connection: _connection,
        transactionScope: _scope,
      );

  final Sqlite3Driver _driver;
  final _ConnectionState _connection;
  final _TransactionScope _scope;

  // Keep ownership until a live child finishes, including unawaited children.
  Future<Result<T, SqlxError>> _invoke<T>(
    Future<Result<T, SqlxError>> Function(Transaction tx) fn,
  ) => runZoned(() async {
    try {
      return await fn(this);
    } finally {
      _deactivate();
      await _scope.nestedDone;
    }
  }, zoneValues: {_connection: _scope});

  @override
  sqlite.Database get database => _driver.database;

  @override
  Driver get driver => _driver.driver;

  @override
  Future<Result<T?, SqlxError>> fetchOptional<T>(
    String sql,
    List<Object?> parameters,
    RowMapper<T> mapper,
  ) {
    return _driver.fetchOptional(sql, parameters, mapper);
  }

  @override
  Future<Result<List<T>, SqlxError>> fetchAll<T>(
    String sql,
    List<Object?> parameters,
    RowMapper<T> mapper,
  ) {
    return _driver.fetchAll(sql, parameters, mapper);
  }

  @override
  Future<Result<T, SqlxError>> fetchOne<T>(
    String sql,
    List<Object?> parameters,
    RowMapper<T> mapper,
  ) {
    return _driver.fetchOne(sql, parameters, mapper);
  }

  @override
  Future<Result<T, SqlxError>> fetchScalar<T>(
    String sql,
    List<Object?> parameters,
  ) {
    return _driver.fetchScalar(sql, parameters);
  }

  @override
  Future<Result<ExecResult, SqlxError>> execute(
    String sql,
    List<Object?> parameters,
  ) {
    return _driver.execute(sql, parameters);
  }

  @override
  Future<Result<T, SqlxError>> transaction<T>(
    Future<Result<T, SqlxError>> Function(Transaction tx) fn,
  ) {
    final error = _driver._closedError();
    if (error != null) return Future.value(Err<T, SqlxError>(error));
    return _scope.runNested(() => _connection.runSavepoint(database, fn));
  }

  @override
  Future<Result<Unit, SqlxError>> close() {
    if (!_scope.active) return Future.value(const Ok<Unit, SqlxError>(unit));
    final error = _driver._closedError(closing: true);
    if (error != null) return Future.value(Err<Unit, SqlxError>(error));
    _deactivate();
    return Future.value(const Ok<Unit, SqlxError>(unit));
  }

  void _deactivate() {
    _scope.active = false;
  }
}
