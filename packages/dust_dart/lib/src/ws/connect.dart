import 'package:web_socket_channel/web_socket_channel.dart';

import '../serde/serde.dart';
import 'connection.dart';
import 'connector_portable.dart' if (dart.library.io) 'connector_io.dart'
    as platform;

/// Opens the transport for a typed connection.
///
/// Native platforms support [headers]. The portable connector rejects nonempty
/// headers because browser WebSockets cannot set arbitrary handshake headers.
/// Applications can inject transport-specific options with their own connector.
typedef WebSocketConnector = WebSocketChannel Function(
  Uri uri, {
  Iterable<String>? protocols,
  Map<String, Object?>? headers,
});

/// Connects [uri] and waits for the handshake before returning a typed socket.
///
/// Headers belong to this WebSocket transport; Dio interceptors do not run.
/// No reconnect, retry, remote acknowledgement or RPC behavior is added.
Future<WebSocketConnection<Send, Receive>> connectWebSocket<Send, Receive>(
  Uri uri, {
  required Serializer<Send, Map<String, Object?>> serializer,
  required Deserializer<Receive, Map<String, Object?>> deserializer,
  WebSocketConnector? connector,
  Iterable<String>? protocols,
  Map<String, Object?>? headers,
}) async {
  final channel = (connector ?? platform.connectWebSocketChannel)(
    uri,
    protocols: protocols,
    headers: headers,
  );
  await channel.ready;
  return WebSocketConnection.fromChannel(
    channel,
    serializer: serializer,
    deserializer: deserializer,
  );
}
