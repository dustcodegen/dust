import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:dust_dart/serde.dart';
import 'package:dust_dart/ws.dart';
import 'package:test/test.dart';

import 'support.dart';

void main() {
  WebSocketConnection<int, int> adapt(TestChannel channel) =>
      WebSocketConnection.fromChannel(
        channel,
        serializer: const IntCodec(),
        deserializer: const IntCodec(),
      );

  test('adapted sends wait for readiness and enqueue in call order', () async {
    final ready = Completer<void>();
    final channel = TestChannel(ready: ready.future);
    addTearDown(channel.dispose);
    final connection = adapt(channel);

    final first = connection.send(1);
    final second = connection.send(2);
    expect(channel.sink.sent, isEmpty);

    ready.complete();
    await connection.ready;
    await Future.wait([first, second]);
    expect(channel.sink.sent, ['{"value":1}', '{"value":2}']);
  });

  test('adapted sends propagate readiness failures', () async {
    final ready = Completer<void>();
    final channel = TestChannel(ready: ready.future);
    addTearDown(channel.dispose);
    final error = StateError('handshake failed');
    final result = expectLater(adapt(channel).send(1), throwsA(same(error)));

    ready.completeError(error);
    await result;
    expect(channel.sink.sent, isEmpty);
  });

  test('send surfaces serializer, JSON encoding and sink failures', () async {
    final channel = TestChannel();
    addTearDown(channel.dispose);
    final error = StateError('serializer failed');
    final failing = WebSocketConnection<int, int>.fromChannel(
      channel,
      serializer: _FailingSerializer(error),
      deserializer: const IntCodec(),
    );
    await expectLater(failing.send(1), throwsA(same(error)));

    final unsupported = WebSocketConnection<Object?, int>.fromChannel(
      channel,
      serializer: const _ObjectSerializer(),
      deserializer: const IntCodec(),
    );
    await expectLater(
      unsupported.send(Object()),
      throwsA(isA<JsonUnsupportedObjectError>()),
    );

    final sinkError = StateError('sink failed');
    channel.sink.addFailure = sinkError;
    await expectLater(adapt(channel).send(1), throwsA(same(sinkError)));
  });

  test('messages preserve order and continue after frame and codec errors',
      () async {
    final channel = TestChannel();
    addTearDown(channel.dispose);
    final connection = adapt(channel);
    final events = <Object?>[];
    final finished = Completer<void>();
    final transportError = StateError('transport failed');
    expect(connection.messages.isBroadcast, isFalse);
    connection.messages.listen(
      events.add,
      onError: (Object error) => events.add(error),
      onDone: finished.complete,
    );

    channel.incoming
      ..add('{"value":1}')
      ..add('{')
      ..add('[1]')
      ..add(Uint8List.fromList([1, 2]))
      ..add('{"value":"bad"}')
      ..addError(transportError)
      ..add('{"value":2}');
    await channel.incoming.close();
    await finished.future;

    expect(events, [
      1,
      isA<JsonDecodeSyntaxError>(),
      isA<JsonDecodeTypeError>(),
      isA<FormatException>(),
      isA<ArgumentError>(),
      same(transportError),
      2,
    ]);
    expect(
      () => connection.messages.listen((_) {}),
      throwsStateError,
    );
  });

  test('protocol override preserves server negotiation and channel fallback',
      () {
    final channel = TestChannel(protocol: 'channel.v1');
    addTearDown(channel.dispose);
    final overridden = WebSocketConnection<int, int>.fromChannel(
      channel,
      serializer: const IntCodec(),
      deserializer: const IntCodec(),
      negotiatedProtocol: 'server.v2',
    );
    expect(overridden.protocol, 'server.v2');
    expect(adapt(channel).protocol, 'channel.v1');

    final noProtocol = TestChannel();
    addTearDown(noProtocol.dispose);
    expect(adapt(noProtocol).protocol, isNull);
  });

  test('close and done delegate without fabricating peer metadata', () async {
    final channel = TestChannel();
    addTearDown(channel.dispose);
    final connection = adapt(channel);
    expect(connection.closeCode, isNull);
    expect(connection.closeReason, isNull);

    await connection.close(4001, 'finished');
    await connection.done;
    expect(channel.sink.requestedCode, 4001);
    expect(channel.sink.requestedReason, 'finished');
    expect(connection.closeCode, isNull);
    expect(connection.closeReason, isNull);

    channel
      ..closeCode = 4002
      ..closeReason = 'peer finished';
    expect(connection.closeCode, 4002);
    expect(connection.closeReason, 'peer finished');
    await expectLater(connection.send(3), throwsStateError);
  });

  test('default close uses the normal application close code', () async {
    final channel = TestChannel();
    addTearDown(channel.dispose);
    await adapt(channel).close();
    expect(channel.sink.requestedCode, 1000);
    expect(channel.sink.requestedReason, isNull);
  });
}

final class _FailingSerializer
    implements Serializer<int, Map<String, Object?>> {
  _FailingSerializer(this.error);

  final Object error;

  @override
  Map<String, Object?> serialize(int value) => throw error;
}

final class _ObjectSerializer
    implements Serializer<Object?, Map<String, Object?>> {
  const _ObjectSerializer();

  @override
  Map<String, Object?> serialize(Object? value) => {'value': value};
}
