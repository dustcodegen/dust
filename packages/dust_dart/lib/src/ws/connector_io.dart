import 'package:web_socket_channel/io.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

/// Opens a native channel with handshake headers and subprotocols.
WebSocketChannel connectWebSocketChannel(
  Uri uri, {
  Iterable<String>? protocols,
  Map<String, Object?>? headers,
}) =>
    IOWebSocketChannel.connect(uri, protocols: protocols, headers: headers);
