import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:server_client_app/websocket_client.dart';
import 'package:server_client_app/websocket_contracts.dart';
import 'package:shopping_app/generated_websocket_example.dart';

void main() {
  testWidgets(
    'generated client exchanges typed messages and closes on dispose',
    (
      tester,
    ) async {
      final server = (await tester.runAsync(_ChatServer.open))!;
      final dio = Dio(BaseOptions(baseUrl: server.baseUrl));
      addTearDown(() async {
        dio.close(force: true);
        await server.close();
      });
      await tester.pumpWidget(
        MaterialApp(home: GeneratedChatExample(client: ChatClient(dio))),
      );
      await _waitForText(tester, 'Connected');
      await tester.tap(find.text('Send hello'));
      await _waitForText(tester, 'echo:hello');

      expect(server.request!.uri.path, '/rooms/lobby');
      expect(server.request!.uri.queryParameters['sequence'], '1');
      expect(
        server.request!.headers.value('authorization'),
        'Bearer fixture-token',
      );
      expect(server.protocol, 'dust.chat.v1');

      await tester.pumpWidget(const SizedBox.shrink());
      await _waitFor(tester, () => server.closed.isCompleted, 'peer close');
    },
  );

  testWidgets('connection opened after widget disposal is closed', (
    tester,
  ) async {
    final server = (await tester.runAsync(
      () => _ChatServer.open(defer: true),
    ))!;
    final dio = Dio(BaseOptions(baseUrl: server.baseUrl));
    addTearDown(() async {
      dio.close(force: true);
      await server.close();
    });
    await tester.pumpWidget(
      MaterialApp(home: GeneratedChatExample(client: ChatClient(dio))),
    );
    await _waitFor(
      tester,
      () => server.requested.isCompleted,
      'upgrade request',
    );
    await tester.pumpWidget(const SizedBox.shrink());
    server.release.complete();
    await _waitFor(tester, () => server.closed.isCompleted, 'peer close');
    expect(tester.takeException(), isNull);
  });
}

const _timeout = Duration(seconds: 5);

Future<void> _waitForText(WidgetTester tester, String text) => _waitFor(
  tester,
  () => find.text(text).evaluate().isNotEmpty,
  'widget text: $text',
);

Future<void> _waitFor(
  WidgetTester tester,
  bool Function() condition,
  String event,
) async {
  final deadline = DateTime.now().add(_timeout);
  while (!condition()) {
    if (DateTime.now().isAfter(deadline)) {
      fail('Did not observe $event');
    }
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 1)),
    );
    await tester.pump();
  }
}

final class _ChatServer {
  _ChatServer(this._server);

  final HttpServer _server;
  final requested = Completer<void>();
  final release = Completer<void>();
  final closed = Completer<void>();
  HttpRequest? request;
  WebSocket? _socket;
  Future<void>? _handler;

  String get baseUrl => 'http://${_server.address.address}:${_server.port}';
  String? get protocol => _socket?.protocol;

  static Future<_ChatServer> open({bool defer = false}) async {
    final server = _ChatServer(
      await HttpServer.bind(InternetAddress.loopbackIPv4, 0),
    );
    if (!defer) server.release.complete();
    server._server.listen((request) {
      server._handler = server._upgrade(request);
    });
    return server;
  }

  Future<void> _upgrade(HttpRequest value) async {
    request = value;
    requested.complete();
    await release.future;
    final socket = await WebSocketTransformer.upgrade(
      value,
      protocolSelector: (_) => 'dust.chat.v1',
    );
    _socket = socket;
    socket.listen(
      (frame) {
        final command = const $ChatCommandDeserializer().deserialize(
          Map<String, Object?>.from(
            jsonDecode(frame as String) as Map<String, dynamic>,
          ),
        );
        final notice = ChatNotice(
          room: 'lobby',
          sequence: 1,
          user: 'Ada',
          text: 'echo:${command.text}',
          protocol: socket.protocol,
        );
        socket.add(jsonEncode(notice.serialize()));
      },
      onDone: () {
        if (!closed.isCompleted) closed.complete();
      },
    );
  }

  Future<void> close() async {
    if (!release.isCompleted) release.complete();
    await _handler;
    await _socket?.close();
    await _server.close(force: true);
  }
}
