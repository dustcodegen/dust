import 'dart:convert';

import 'package:web_socket_channel/web_socket_channel.dart';

import '../fp/result.dart';
import '../serde/json_result.dart';
import '../serde/serde.dart';

/// One JSON object per text frame, with independent send and receive types.
///
/// [messages] owns the channel's single subscription. Canceling that subscription
/// does not promise to cancel socket I/O; call [close] to close the connection.
final class WebSocketConnection<Send, Receive> {
  /// Adapts [channel] with Dust's generated directional Serde contracts.
  ///
  /// The channel may still be connecting. [send] waits for [ready] before it
  /// serializes or enqueues a frame. Servers can supply [negotiatedProtocol]
  /// when their upgrade adapter reports the protocol separately from [channel].
  WebSocketConnection.fromChannel(
    WebSocketChannel channel, {
    required Serializer<Send, Map<String, Object?>> serializer,
    required Deserializer<Receive, Map<String, Object?>> deserializer,
    String? negotiatedProtocol,
  })  : _channel = channel,
        _serializer = serializer,
        _negotiatedProtocol = negotiatedProtocol,
        messages = channel.stream.map(
          (frame) => _decode(frame, deserializer),
        );

  final WebSocketChannel _channel;
  final Serializer<Send, Map<String, Object?>> _serializer;
  final String? _negotiatedProtocol;

  /// Incoming values in frame order, on one single-subscription stream.
  ///
  /// Invalid JSON, non-object roots, binary frames and deserializer failures
  /// arrive as stream errors. Later valid frames can still be received.
  final Stream<Receive> messages;

  /// Completes once the connection is ready, or fails with its handshake error.
  Future<void> get ready => _channel.ready;

  /// Serializes and enqueues [value] after [ready] completes.
  ///
  /// Completion means the frame was enqueued, not acknowledged by the peer.
  Future<void> send(Send value) async {
    await ready;
    _channel.sink.add(jsonEncode(_serializer.serialize(value)));
  }

  /// Closes the connection with an optional application close code and reason.
  ///
  /// [code] must be 1000 or in the application range 3000–4999.
  Future<void> close([int code = 1000, String? reason]) =>
      _channel.sink.close(code, reason);

  /// Completes when the underlying channel's outgoing sink finishes.
  ///
  /// Observing a peer close can require consuming [messages]. Awaiting only
  /// this future does not promise to read incoming frames or detect peer close.
  Future<void> get done => _channel.sink.done;

  /// The subprotocol agreed during the handshake, if any.
  String? get protocol => _negotiatedProtocol ?? _channel.protocol;

  /// The close code reported by the channel after the peer closes.
  int? get closeCode => _channel.closeCode;

  /// The close reason reported by the channel after the peer closes.
  String? get closeReason => _channel.closeReason;
}

Receive _decode<Receive>(
  Object? frame,
  Deserializer<Receive, Map<String, Object?>> deserializer,
) {
  if (frame is! String) {
    throw FormatException('Expected a JSON object text WebSocket frame.');
  }
  return switch (decodeJsonObjectResult(frame)) {
    Ok(:final value) => deserializer.deserialize(value),
    Err(:final error) => throw error,
  };
}
