import 'dart:io';

/// A real echo transport for native and portable connector tests.
final class LoopbackServer {
  LoopbackServer._(this._server) {
    _server.listen((request) async {
      authorization.add(request.headers.value(HttpHeaders.authorizationHeader));
      final socket = await WebSocketTransformer.upgrade(
        request,
        protocolSelector: (protocols) =>
            protocols.contains('chat.v1') ? 'chat.v1' : null,
      );
      _sockets.add(socket);
      socket.listen((frame) {
        if (frame == 'close' || frame == '{"value":99}') {
          socket.close(4009, 'peer finished');
        } else {
          socket.add(frame);
        }
      });
    });
  }

  final HttpServer _server;
  final _sockets = <WebSocket>[];

  /// Authorization observed during each handshake.
  final authorization = <String?>[];

  /// The local server URI.
  Uri get uri => Uri.parse('ws://127.0.0.1:${_server.port}/echo');

  /// Starts listening on an ephemeral loopback port.
  static Future<LoopbackServer> start() async => LoopbackServer._(
        await HttpServer.bind(InternetAddress.loopbackIPv4, 0),
      );

  /// Closes upgraded sockets and the HTTP listener.
  Future<void> close() async {
    await Future.wait(_sockets.map((socket) => socket.close()));
    await _server.close(force: true);
  }
}
