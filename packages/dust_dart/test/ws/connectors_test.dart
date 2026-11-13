@TestOn('vm')
library;

import 'dart:io';

import 'package:dust_dart/src/ws/connector_io.dart' as native;
import 'package:dust_dart/src/ws/connector_portable.dart' as portable;
import 'package:dust_dart/ws.dart';
import 'package:test/test.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import 'loopback.dart';
import 'support.dart';

void main() {
  late LoopbackServer server;

  setUp(() async => server = await LoopbackServer.start());
  tearDown(() => server.close());

  test('native connector transmits headers and negotiates a protocol',
      () async {
    final channel = native.connectWebSocketChannel(
      server.uri,
      headers: {'authorization': 'native-token'},
      protocols: ['chat.v1'],
    );
    await channel.ready;
    final frames = channel.stream.toList();
    channel.sink
      ..add('first')
      ..add('second')
      ..add('close');

    expect(await frames, ['first', 'second']);
    await channel.sink.done;
    expect(server.authorization, ['native-token']);
    expect(channel.protocol, 'chat.v1');
    expect(channel.closeCode, 4009);
    expect(channel.closeReason, 'peer finished');
    await channel.sink.close();
  });

  for (final headers in [null, <String, Object?>{}]) {
    test('portable connector echoes with headers=$headers', () async {
      final channel = portable.connectWebSocketChannel(
        server.uri,
        headers: headers,
        protocols: ['chat.v1'],
      );
      await channel.ready;
      final frames = channel.stream.toList();
      channel.sink
        ..add('{"value":3}')
        ..add('close');

      expect(await frames, ['{"value":3}']);
      await channel.sink.done;
      expect(server.authorization, [null]);
      expect(channel.protocol, 'chat.v1');
      expect(channel.closeCode, 4009);
      await channel.sink.close();
    });
  }

  test('portable connector rejects arbitrary headers before connecting', () {
    expect(
      () => portable.connectWebSocketChannel(
        server.uri,
        headers: {'authorization': 'cannot-be-sent'},
      ),
      throwsA(isA<UnsupportedError>()),
    );
    expect(server.authorization, isEmpty);
  });

  test('default platform connector provides a ready typed connection',
      () async {
    final connection = await connectWebSocket<int, int>(
      server.uri,
      serializer: const IntCodec(),
      deserializer: const IntCodec(),
      protocols: ['chat.v1'],
      headers: {'authorization': 'default-token'},
    );
    final values = connection.messages.toList();
    await connection.send(1);
    await connection.send(2);
    await connection.send(99);

    expect(await values, [1, 2]);
    await connection.done;
    expect(server.authorization, ['default-token']);
    expect(connection.protocol, 'chat.v1');
    expect(connection.closeCode, 4009);
    expect(connection.closeReason, 'peer finished');
    await connection.close();
  });

  test('failed native handshake propagates through connectWebSocket', () async {
    final rejecting = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    addTearDown(() => rejecting.close(force: true));
    rejecting.listen((request) {
      request.response.statusCode = HttpStatus.forbidden;
      request.response.close();
    });
    late WebSocketChannel channel;
    final pending = connectWebSocket<int, int>(
      Uri.parse('ws://127.0.0.1:${rejecting.port}/rejected'),
      serializer: const IntCodec(),
      deserializer: const IntCodec(),
      connector: (uri, {protocols, headers}) =>
          channel = native.connectWebSocketChannel(uri, protocols: protocols),
    );

    await expectLater(pending, throwsA(isA<WebSocketChannelException>()));
    await expectLater(
      channel.stream.drain<void>(),
      throwsA(isA<WebSocketChannelException>()),
    );
  });
}
