part of 'sqlite_pool.dart';

/// Asynchronous unchecked SQL for a generated background database facade.
///
/// Native resources stay in the worker; returned rows and mappers stay on the
/// caller. This adapter does not bypass transaction ownership checks.
final class Sqlite3WorkerUnsafeSql implements UnsafeSql {
  /// Wraps one open worker connection without exposing its native handle.
  const Sqlite3WorkerUnsafeSql(this._driver);

  final Sqlite3WorkerDriver _driver;

  @override
  Future<Result<List<Row>, SqlxError>> fetch(
    String sql,
    List<Object?> parameters,
  ) => _driver._rows(sql, parameters);

  @override
  Future<Result<List<T>, SqlxError>> fetchAs<T>(
    String sql,
    List<Object?> parameters,
    RowMapper<T> mapper,
  ) => _driver.fetchAll(sql, parameters, mapper);

  @override
  Future<Result<ExecResult, SqlxError>> execute(
    String sql,
    List<Object?> parameters,
  ) => _driver.execute(sql, parameters);
}
