@TestOn('vm')
library;

import 'dart:io';
import 'dart:isolate';

import 'package:dio/dio.dart';
import 'package:dust_server/server.dart' show Router, ServerHandle, serve;
import 'package:server_client_app/websocket_client.dart';
import 'package:server_client_app/websocket_contracts.dart';
import 'package:server_client_server/websocket.dart';
import 'package:test/test.dart';

void main() {
  late Chat controller;
  late ServerHandle server;
  late Dio dio;
  late ChatClient client;
  late List<Object> errors;

  setUp(() async {
    controller = Chat();
    errors = [];
    final app = Router(onError: (error, stack) => errors.add(error))
      ..mount('/', controller.routes().call);
    server = await serve(app, InternetAddress.loopbackIPv4, 0);
    dio = Dio(
      BaseOptions(
        baseUrl: 'http://${server.address.host}:${server.port}',
        connectTimeout: const Duration(seconds: 5),
        receiveTimeout: const Duration(seconds: 5),
      ),
    );
    client = ChatClient(dio);
  });

  tearDown(() async {
    dio.close(force: true);
    await server.close();
    expect(errors, isEmpty);
  });

  test('documented client entrypoint exchanges a message and exits', () async {
    final source = await Isolate.resolvePackageUri(
      Uri.parse('package:server_client_server/websocket.dart'),
    );
    final example = source!.resolve('../example/websocket_client.dart');
    final process = await Process.run(Platform.resolvedExecutable, [
      'run',
      example.toFilePath(),
      dio.options.baseUrl,
    ]);

    expect(process.exitCode, 0, reason: '${process.stderr}');
    expect(process.stdout.toString().trim(), 'Ada in lobby: hello');
    expect(controller.sessionsStarted, 1);
  });

  test('generated client and server exchange typed frames over loopback',
      () async {
    const room = 'dust room/%25';
    final socket = await client
        .chat(
          room,
          sequence: 7,
          token: 'Bearer fixture-token',
        )
        .timeout(const Duration(seconds: 5));
    try {
      final received = socket.messages.take(2).toList();
      await socket.send(const ChatCommand(text: 'first'));
      await socket.send(const ChatCommand(text: 'second'));

      expect(
        await received.timeout(const Duration(seconds: 5)),
        const [
          ChatNotice(
            room: room,
            sequence: 7,
            user: 'Ada',
            text: 'first',
            protocol: 'dust.chat.v1',
          ),
          ChatNotice(
            room: room,
            sequence: 7,
            user: 'Ada',
            text: 'second',
            protocol: 'dust.chat.v1',
          ),
        ],
      );
      expect(socket.protocol, 'dust.chat.v1');
      expect(controller.sessionsStarted, 1);
    } finally {
      await socket.close().timeout(const Duration(seconds: 5));
    }
  });

  test('query, header and authentication failures reject before upgrade',
      () async {
    Future<Response<Object?>> handshake({
      String sequence = '7',
      String? token = 'Bearer fixture-token',
    }) {
      return dio.get<Object?>(
        '/rooms/general',
        queryParameters: {'sequence': sequence},
        options: Options(
          headers: {
            'connection': 'Upgrade',
            'upgrade': 'websocket',
            'sec-websocket-version': '13',
            'sec-websocket-key': 'dGhlIHNhbXBsZSBub25jZQ==',
            if (token != null) 'authorization': token,
          },
          validateStatus: (_) => true,
        ),
      );
    }

    expect((await handshake(sequence: 'bad')).statusCode, 400);
    expect((await handshake(token: null)).statusCode, 400);
    expect((await handshake(token: 'Bearer wrong')).statusCode, 401);
    expect(controller.sessionsStarted, 0);
  });
}
