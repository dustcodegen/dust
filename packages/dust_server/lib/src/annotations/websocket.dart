import 'verb.dart';

/// Declares a typed WebSocket route whose handshake uses `GET`.
///
/// The handler returns `void` or `Future<void>` and accepts exactly one
/// `WebSocketConnection<Send, Receive>` alongside request-parts extractors.
final class WS extends Verb {
  /// Declares a WebSocket upgrade at [path].
  const WS(
    super.path, {
    this.protocols,
    this.allowedOrigins,
    super.summary,
    super.description,
    super.tags,
    super.middleware,
    super.operationId,
    super.deprecated,
    super.hidden,
  });

  /// Protocols offered during the handshake; negotiation may select none.
  final List<String>? protocols;

  /// Permitted browser origins; requests without an Origin remain permitted.
  final List<String>? allowedOrigins;

  @override
  String get method => 'GET';
}
