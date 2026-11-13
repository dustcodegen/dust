import 'dart:async';

import 'package:dust_dart/ws.dart';
import 'package:test/test.dart';

import 'support.dart';

void main() {
  test('connect forwards options and returns only after readiness', () async {
    final ready = Completer<void>();
    final channel = TestChannel(ready: ready.future, protocol: 'chat.v1');
    addTearDown(channel.dispose);
    final uri = Uri.parse('wss://example.test/rooms/one');
    var connected = false;
    final pending = connectWebSocket<int, int>(
      uri,
      serializer: const IntCodec(),
      deserializer: const IntCodec(),
      protocols: const ['chat.v1'],
      headers: const {'authorization': 'test-token'},
      connector: (actualUri, {protocols, headers}) {
        expect(actualUri, uri);
        expect(protocols, ['chat.v1']);
        expect(headers, {'authorization': 'test-token'});
        return channel;
      },
    ).then((connection) {
      connected = true;
      return connection;
    });

    await Future<void>.value();
    expect(connected, isFalse);
    ready.complete();
    final connection = await pending;
    expect(connected, isTrue);
    expect(connection.protocol, 'chat.v1');
    await connection.send(7);
    expect(channel.sink.sent, ['{"value":7}']);
  });

  test('connect propagates a failed handshake', () async {
    final ready = Completer<void>();
    final channel = TestChannel(ready: ready.future);
    addTearDown(channel.dispose);
    final error = StateError('rejected handshake');
    final pending = connectWebSocket<int, int>(
      Uri.parse('ws://example.test'),
      serializer: const IntCodec(),
      deserializer: const IntCodec(),
      connector: (uri, {protocols, headers}) => channel,
    );
    final checked = expectLater(pending, throwsA(same(error)));
    ready.completeError(error);
    await checked;
    expect(channel.sink.sent, isEmpty);
  });

  test('connect propagates synchronous connector failures', () async {
    final error = UnsupportedError('transport unavailable');
    await expectLater(
      connectWebSocket<int, int>(
        Uri.parse('ws://example.test'),
        serializer: const IntCodec(),
        deserializer: const IntCodec(),
        connector: (uri, {protocols, headers}) => throw error,
      ),
      throwsA(same(error)),
    );
  });
}
