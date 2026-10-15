part of 'sqlite_pool.dart';

extension _WorkerTransactions on _WorkerExecutor {
  Future<Result<T, SqlxError>> _runWorkerTransaction<T>(
    Future<Result<T, SqlxError>> Function(Transaction tx) fn,
  ) async {
    final transport = _connection.transport;
    final begin = await transport.request(['begin', _txId]);
    if (begin case Err(:final error)) return Err(error);
    final tx = _WorkerTransaction(_connection, begin.unwrap()! as int);
    final aborted = Completer<Result<T, SqlxError>>();
    final subscription = transport.failures.stream.listen(
      (error) => aborted.complete(Err(error)),
    );
    Result<T, SqlxError> result;
    Future<Result<T, SqlxError>> invoke() => runZoned(() async {
      try {
        return await fn(tx);
      } finally {
        tx._scope!.active = false;
        await tx._scope.nestedDone;
      }
    }, zoneValues: {_connection: tx._scope});
    try {
      result = await Future.any([invoke(), aborted.future]);
    } catch (error) {
      result = Err(
        _sqliteTransactionError(
          'SQLite transaction failed.',
          cause: error,
          operation: 'transaction callback',
        ),
      );
    } finally {
      tx._scope!.active = false;
      await subscription.cancel();
    }
    final finish = await transport.request([
      'finish',
      tx._txId,
      result.match(ok: (_) => null, err: _workerError),
    ]);
    // Values and callback errors stay local; cleanup failures take precedence.
    return finish.match(ok: (_) => result, err: Err.new);
  }
}

final class _WorkerTransaction extends _WorkerExecutor implements Transaction {
  _WorkerTransaction(_WorkerConnection connection, int txId)
    : super(connection, _TransactionScope(), txId);

  @override
  Future<Result<Unit, SqlxError>> close() {
    if (!_scope!.active) return Future.value(const Ok(unit));
    return _access('close', () async {
      _scope.active = false;
      return const Ok(unit);
    });
  }
}
