import 'package:dust_server/server.dart';
import 'package:server_client_app/websocket_contracts.dart' as contracts;

part 'websocket.g.dart';

/// Generated typed upgrade routes with required pre-upgrade authentication.
@Controller('/rooms', tags: ['public'])
class Chat with _$Chat {
  /// Number of session handlers actually entered after successful upgrades.
  int sessionsStarted = 0;

  /// Exchanges client commands and server notices on one connection.
  @WS(
    '/{room}',
    protocols: ['dust.chat.v1'],
    allowedOrigins: ['https://chat.example'],
  )
  Future<void> chat(
    @Path() String room,
    WebSocketConnection<contracts.ChatNotice, contracts.ChatCommand> socket, {
    @Query('sequence') required int sequence,
    @Header('authorization') required String token,
    @Extract(ChatAuth) required String user,
  }) async {
    sessionsStarted++;
    await for (final command in socket.messages) {
      await socket.send(
        contracts.ChatNotice(
          room: room,
          sequence: sequence,
          user: user,
          text: command.text,
          protocol: socket.protocol,
        ),
      );
    }
  }
}

/// Stateless example authentication executed before accepting the handshake.
final class ChatAuth implements FromRequestParts<String> {
  /// Creates the generated handler's authentication extractor.
  const ChatAuth();

  @override
  Future<Result<String, Rejection>> extract(Request request) async {
    final token = await const BearerTokenExtractable().extract(request);
    return switch (token) {
      Err(:final error) => Err(error),
      Ok(value: 'fixture-token') => const Ok('Ada'),
      Ok() => const Err(Rejection.unauthorized('invalid fixture token')),
    };
  }
}
