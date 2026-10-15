part of 'sqlite_pool.dart';

final class _WorkerTransport {
  _WorkerTransport(this._send);
  final void Function(SendPort, Object?) _send;
  final _events = ReceivePort();
  final _ready = Completer<Result<SendPort, SqlxError>>();
  final _pending = <int, Completer<Result<Object?, SqlxError>>>{};
  final failures = StreamController<SqlxError>.broadcast(sync: true);
  var _nextId = 0;
  Isolate? _isolate;
  late SendPort _commands;
  SqlxError? _terminal;

  Future<void> start(
    SqliteConnectOptions options,
    Map<String, String>? migrations,
    void Function(List<Object?>) entrypoint,
  ) async {
    _events.listen(_receive);
    try {
      _isolate = await Isolate.spawn(
        entrypoint,
        <Object?>[_events.sendPort, _workerOptions(options), migrations],
        onError: _events.sendPort,
        onExit: _events.sendPort,
        debugName: 'Dust SQLite',
      );
      // Startup can fail before spawn returns its handle.
      if (_terminal != null) _isolate!.kill(priority: Isolate.immediate);
      _commands = (await _ready.future).unwrapOrElse((error) => throw error);
    } on SqlxError {
      rethrow;
    } catch (error) {
      final failure = _sqliteConnectionError(
        'SQLite worker spawn failed.',
        cause: error,
        operation: 'worker spawn',
      );
      _fail(failure);
      throw failure;
    }
  }

  void _receive(Object? message) {
    if (_terminal != null) return;
    try {
      _reply(message);
    } catch (error) {
      _fail(
        _sqliteConnectionError(
          'Invalid SQLite worker reply.',
          cause: error,
          operation: 'worker protocol',
        ),
      );
    }
  }

  void _reply(Object? message) {
    if (message == null || (message is List && message.length == 2)) {
      _fail(
        _sqliteConnectionError(
          'SQLite worker stopped. Close and reopen; do not replay uncertain writes.',
          cause: message,
          operation: 'worker exit',
        ),
      );
      return;
    }
    final reply = message as List<Object?>;
    final id = reply[0]! as int;
    final result = reply[1] == true
        ? Ok<Object?, SqlxError>(reply[2])
        : Err<Object?, SqlxError>(_readWorkerError(reply[2]));
    if (id == 0) {
      if (result case Err(:final error)) {
        _fail(error);
      } else {
        _ready.complete(result.map((value) => value! as SendPort));
      }
    } else {
      final pending = _pending.remove(id);
      if (pending == null) throw StateError('Unknown SQLite worker reply id.');
      pending.complete(result);
    }
  }

  Future<Result<Object?, SqlxError>> request(List<Object?> command) {
    final terminal = _terminal;
    if (terminal != null) return Future.value(Err(terminal));
    final id = ++_nextId;
    final done = Completer<Result<Object?, SqlxError>>();
    _pending[id] = done;
    try {
      _send(_commands, <Object?>[id, ...command]);
    } catch (error) {
      _fail(
        _sqliteConnectionError(
          'SQLite worker request could not be sent.',
          cause: error,
          operation: 'worker send',
        ),
      );
    }
    return done.future;
  }

  void _fail(SqlxError error) {
    if (_terminal != null) return;
    _terminal = error;
    failures.add(error);
    unawaited(failures.close());
    if (!_ready.isCompleted) _ready.complete(Err(error));
    for (final pending in _pending.values) {
      pending.complete(Err(error));
    }
    _pending.clear();
    _events.close();
    _isolate?.kill(priority: Isolate.immediate);
  }

  void dispose() {
    _terminal ??= _sqliteConnectionError(
      'SQLite worker connection is closed.',
      operation: 'checkOpen',
    );
    _events.close();
    unawaited(failures.close());
  }
}

void _sendWorkerMessage(SendPort port, Object? message) => port.send(message);
