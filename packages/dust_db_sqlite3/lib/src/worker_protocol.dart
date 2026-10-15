part of 'sqlite_pool.dart';

// Only primitives, copied lists/maps/bytes and the startup SendPort cross the
// isolate boundary. Native resources, callbacks and mappers never do.
Map<String, Object?> _workerError(SqlxError error) => {
  'category': error.category.name,
  'message': error.message,
  'operation': error.operation,
  'kind': error.kind?.name,
  'cause': _workerDiagnostic(error.cause),
};

String? _workerDiagnostic(Object? cause) {
  try {
    return cause?.toString();
  } catch (_) {
    return 'SQLite diagnostic could not be converted to text.';
  }
}

SqlxError _readWorkerError(Object? message) {
  final data = message! as Map<Object?, Object?>;
  return SqlxDriverError(
    data['message']! as String,
    category: SqlxErrorCategory.values.byName(data['category']! as String),
    driver: Driver.sqlite3,
    operation: data['operation'] as String?,
    kind: data['kind'] == null
        ? null
        : SqlxErrorKind.values.byName(data['kind']! as String),
    cause: data['cause'],
  );
}

Map<String, Object?> _workerOptions(SqliteConnectOptions options) => {
  'path': options.path,
  'memory': options.inMemory,
  'create': options.createIfMissing,
  'readOnly': options.readOnly,
  'timeout': options.busyTimeout?.inMicroseconds,
  'foreignKeys': options.foreignKeys,
  'journal': options.journalMode?.name,
  'synchronous': options.synchronous?.name,
  'pragmas': Map<String, Object>.of(options.pragmas),
  'baseline': options.baselineLegacyMigrations,
};

SqliteConnectOptions _readWorkerOptions(Object? message) {
  final data = message! as Map<Object?, Object?>;
  final options = SqliteConnectOptions(
    createIfMissing: data['create']! as bool,
    readOnly: data['readOnly']! as bool,
    busyTimeout: data['timeout'] == null
        ? null
        : Duration(microseconds: data['timeout']! as int),
    foreignKeys: data['foreignKeys'] as bool?,
    journalMode: data['journal'] == null
        ? null
        : SqliteJournalMode.values.byName(data['journal']! as String),
    synchronous: data['synchronous'] == null
        ? null
        : SqliteSynchronousMode.values.byName(data['synchronous']! as String),
    pragmas: (data['pragmas']! as Map).cast<String, Object>(),
    baselineLegacyMigrations: data['baseline'] == true,
  );
  if (data['memory'] != true) return options._withPath(data['path']! as String);
  return SqliteConnectOptions.memory(
    busyTimeout: options.busyTimeout,
    foreignKeys: options.foreignKeys,
    journalMode: options.journalMode,
    synchronous: options.synchronous,
    pragmas: options.pragmas,
    baselineLegacyMigrations: options.baselineLegacyMigrations,
  );
}
