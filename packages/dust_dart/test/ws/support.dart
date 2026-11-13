import 'dart:async';

import 'package:dust_dart/serde.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

/// A small object codec for transport behavior tests.
final class IntCodec implements SerDeCodec<int, Map<String, Object?>> {
  /// Creates the codec.
  const IntCodec();

  @override
  Map<String, Object?> serialize(int value) => {'value': value};

  @override
  int deserialize(Map<String, Object?> json) =>
      JsonHelper.as<int>(json['value'], 'value', 'int');
}

/// A channel whose readiness and incoming frames are controlled by a test.
final class TestChannel implements WebSocketChannel {
  /// Creates a channel, ready immediately unless [ready] is supplied.
  TestChannel({Future<void>? ready, this.protocol})
      : ready = ready ?? Future<void>.value();

  /// Frames delivered to the typed connection.
  final incoming = StreamController<Object?>();

  @override
  final Future<void> ready;

  @override
  final TestSink sink = TestSink();

  @override
  final String? protocol;

  @override
  int? closeCode;

  @override
  String? closeReason;

  @override
  Stream<Object?> get stream => incoming.stream;

  /// Finishes both sides without requiring a listener on buffered frames.
  void dispose() {
    unawaited(incoming.close());
    unawaited(sink.close());
  }

  @override
  dynamic noSuchMethod(Invocation invocation) =>
      throw UnsupportedError('Unexpected channel operation: $invocation');
}

/// A recording sink with the WebSocket close arguments.
final class TestSink implements WebSocketSink {
  /// Enqueued outgoing frames.
  final sent = <Object?>[];

  final _done = Completer<void>();

  /// Error thrown by [add], when set by a test.
  Object? addFailure;

  /// Close code requested by the local connection.
  int? requestedCode;

  /// Close reason requested by the local connection.
  String? requestedReason;

  @override
  Future<void> get done => _done.future;

  @override
  void add(Object? value) {
    if (addFailure case final failure?) throw failure;
    if (_done.isCompleted) throw StateError('Sink is closed.');
    sent.add(value);
  }

  @override
  void addError(Object error, [StackTrace? stackTrace]) => throw error;

  @override
  Future<void> addStream(Stream<Object?> stream) => stream.forEach(add);

  @override
  Future<void> close([int? closeCode, String? closeReason]) {
    requestedCode = closeCode;
    requestedReason = closeReason;
    if (!_done.isCompleted) _done.complete();
    return done;
  }
}
