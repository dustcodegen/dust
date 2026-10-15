part of 'sqlite_pool.dart';

extension _WorkerOperations on _WorkerExecutor {
  Future<Result<Object?, SqlxError>> _statement(
    String sql,
    List<Object?> parameters,
    String operation,
  ) => _access(sql, () {
    final captured = _guardSqlite(sql, 'SQLite parameter capture failed.', () {
      final bound = _encodeListParameters(parameters);
      for (final value in bound) {
        if (value != null &&
            value is! num &&
            value is! bool &&
            value is! String &&
            value is! Uint8List) {
          throw _sqliteQueryError(
            'Unsupported SQLite bind type ${value.runtimeType}.',
            operation: sql,
          );
        }
      }
      return bound;
    });
    return captured.match(
      ok: (bound) =>
          _connection.transport.request([operation, _txId, sql, bound]),
      err: (error) => Future.value(Err(error)),
    );
  });

  Future<Result<List<Sqlite3Row>, SqlxError>> _rows(
    String sql,
    List<Object?> parameters,
  ) async => (await _statement(sql, parameters, 'select')).map((message) {
    final data = message! as List<Object?>;
    final names = (data[0]! as List).cast<String>();
    final index = sqliteColumnIndex(names);
    return [
      for (final row in data[1]! as List)
        Sqlite3Row._shared((row as List).cast<Object?>(), names, index),
    ];
  });

  Future<Result<T?, SqlxError>> _fetchOptional<T>(
    String sql,
    List<Object?> parameters,
    RowMapper<T> mapper,
  ) async => (await _rows(sql, parameters)).andThen(
    (rows) => rows.isEmpty
        ? Ok<T?, SqlxError>(null)
        : _mapRow<T?>(sql, rows.first, mapper),
  );

  Future<Result<T, SqlxError>> _fetchOne<T>(
    String sql,
    List<Object?> parameters,
    RowMapper<T> mapper,
  ) async => (await _rows(sql, parameters)).andThen((rows) {
    if (rows.isEmpty) return Err(_sqliteNoRows(sql));
    if (rows.length > 1) {
      return Err(
        _sqliteTooManyRows(expected: 1, actual: rows.length, query: sql),
      );
    }
    return _mapRow(sql, rows.first, mapper);
  });

  Future<Result<List<T>, SqlxError>> _fetchAll<T>(
    String sql,
    List<Object?> parameters,
    RowMapper<T> mapper,
  ) async => (await _rows(sql, parameters)).andThen((rows) {
    final mapped = <T>[];
    for (final row in rows) {
      final result = _mapRow(sql, row, mapper);
      if (result case Err(:final error)) return Err(error);
      mapped.add(result.unwrap());
    }
    return Ok(mapped);
  });

  Future<Result<T, SqlxError>> _fetchScalar<T>(
    String sql,
    List<Object?> parameters,
  ) async => (await _rows(sql, parameters)).andThen((rows) {
    if (rows.isEmpty) {
      return null is T ? Ok<T, SqlxError>(null as T) : Err(_sqliteNoRows(sql));
    }
    if (rows.length > 1) {
      return Err(
        _sqliteTooManyRows(expected: 1, actual: rows.length, query: sql),
      );
    }
    return _scalarRow<T>(sql, rows.first);
  });

  Future<Result<ExecResult, SqlxError>> _execute(
    String sql,
    List<Object?> parameters,
  ) async => (await _statement(sql, parameters, 'execute')).map((message) {
    final data = message! as List<Object?>;
    return ExecResult(
      rowsAffected: data[0]! as int,
      lastInsertId: data[1] as int?,
    );
  });
}
