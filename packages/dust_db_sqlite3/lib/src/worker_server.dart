part of 'sqlite_pool.dart';

void _sqliteWorkerMain(List<Object?> arguments) => runSqlite3Worker(arguments);

/// Package-internal worker startup seam, hidden by the package entrypoint.
///
/// [open] is created inside the worker and never crosses an isolate boundary.
void runSqlite3Worker(
  List<Object?> arguments, {
  Sqlite3Driver Function(SqliteConnectOptions, Map<String, String>?)? open,
}) {
  final replies = arguments[0]! as SendPort;
  try {
    final options = _readWorkerOptions(arguments[1]);
    final migrations = (arguments[2] as Map?)?.cast<String, String>();
    final driver = open == null
        ? Sqlite3Driver.connect(options, migrations: migrations)
        : open(options, migrations);
    final commands = ReceivePort();
    final server = _WorkerServer(driver, replies, commands);
    commands.listen(
      (message) => unawaited(server.handle(message! as List<Object?>)),
    );
    replies.send(<Object?>[0, true, commands.sendPort]);
  } on SqlxError catch (error) {
    replies.send(<Object?>[0, false, _workerError(error)]);
  }
}

final class _WorkerServer {
  _WorkerServer(this.driver, this.replies, this.commands);
  final Sqlite3Driver driver;
  final SendPort replies;
  final ReceivePort commands;
  final _transactions = <int, _WorkerNativeTransaction>{};

  void reply(int id, Result<Object?, SqlxError> result) => replies.send(
    result.match(
      ok: (value) => <Object?>[id, true, value],
      err: (error) => <Object?>[id, false, _workerError(error)],
    ),
  );

  Future<void> handle(List<Object?> message) async {
    final id = message[0]! as int;
    final operation = message[1]! as String;
    final txId = message[2] as int?;
    final owner = txId == null ? null : _transactions[txId];
    if (txId != null && owner == null) {
      reply(
        id,
        Err(
          _sqliteTransactionError(
            'SQLite transaction is closed.',
            operation: operation,
          ),
        ),
      );
      return;
    }
    final executor = owner?.executor ?? driver;
    Future<void> dispatch() async {
      if (operation == 'begin') {
        await begin(id, executor);
      } else if (operation == 'finish') {
        owner!.finishId = id;
        owner.callbackError = message[3] == null
            ? null
            : _readWorkerError(message[3]);
        owner.finished.complete(
          owner.callbackError == null
              ? const Ok(unit)
              : Err(owner.callbackError!),
        );
      } else if (operation == 'close') {
        final result = await driver.close();
        reply(id, result.map((_) => null));
        commands.close();
      } else {
        final native = executor is Sqlite3Driver
            ? executor
            : (executor as _SingleConnectionPool)._driver;
        final sql = message[3]! as String;
        final parameters = message[4]! as List<Object?>;
        if (operation == 'select') {
          reply(
            id,
            native
                ._selectResult(sql, parameters)
                .map((rows) => <Object?>[rows.columnNames, rows.rows]),
          );
        } else {
          reply(
            id,
            native
                ._executeResult(sql, parameters)
                .map(
                  (result) => <Object?>[
                    result.rowsAffected,
                    result.lastInsertId,
                  ],
                ),
          );
        }
      }
    }

    if (owner == null) {
      await dispatch();
    } else {
      await owner.zone.run(dispatch);
    }
  }

  Future<void> begin(int id, Executor executor) async {
    var entered = false;
    final result = await executor.transaction<Unit>((tx) {
      entered = true;
      final owner = _WorkerNativeTransaction(tx, Zone.current);
      _transactions[id] = owner;
      reply(id, Ok(id));
      return owner.finished.future;
    });
    final owner = _transactions.remove(id);
    if (!entered) {
      reply(id, result.map((_) => null));
    } else {
      final callbackFailed =
          result.isErr && identical(result.unwrapErr(), owner!.callbackError);
      reply(
        owner!.finishId!,
        callbackFailed ? const Ok(null) : result.map((_) => null),
      );
    }
  }
}

final class _WorkerNativeTransaction {
  _WorkerNativeTransaction(this.executor, this.zone);
  final Transaction executor;
  final Zone zone;
  final finished = Completer<Result<Unit, SqlxError>>();
  int? finishId;
  SqlxError? callbackError;
}
