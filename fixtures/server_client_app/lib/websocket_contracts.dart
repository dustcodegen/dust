import 'package:dust_dart/serde.dart';

part 'websocket_contracts.g.dart';

/// One command sent by the generated client to the server.
@Derive([Eq(), Serialize(), Deserialize()])
final class ChatCommand with _$ChatCommand {
  /// Creates a typed chat command.
  const ChatCommand({required this.text});

  /// Message text provided by the client.
  final String text;
}

/// Server acknowledgement carrying the validated handshake context.
@Derive([Eq(), Serialize(), Deserialize()])
final class ChatNotice with _$ChatNotice {
  /// Creates a typed server acknowledgement.
  const ChatNotice({
    required this.room,
    required this.sequence,
    required this.user,
    required this.text,
    this.protocol,
  });

  /// Decoded room path parameter.
  final String room;

  /// Typed query parameter selected before the upgrade.
  final int sequence;

  /// Identity produced by the required authentication extractor.
  final String user;

  /// Text received through the client command codec.
  final String text;

  /// Subprotocol negotiated during the handshake, when one matched.
  final String? protocol;
}
