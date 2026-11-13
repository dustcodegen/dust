import 'package:web_socket_channel/web_socket_channel.dart';

/// Opens a portable channel without silently dropping unsupported headers.
WebSocketChannel connectWebSocketChannel(
  Uri uri, {
  Iterable<String>? protocols,
  Map<String, Object?>? headers,
}) {
  if (headers?.isNotEmpty ?? false) {
    throw UnsupportedError(
      'Arbitrary WebSocket headers require a native or custom connector.',
    );
  }
  return WebSocketChannel.connect(uri, protocols: protocols);
}
