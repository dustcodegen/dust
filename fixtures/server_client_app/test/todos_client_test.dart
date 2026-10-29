import 'dart:convert';
import 'dart:io';

import 'package:dio/dio.dart';
import 'package:server_client_app/server_client_app.dart';
import 'package:test/test.dart';

void main() {
  late HttpServer server;
  late TodosClient client;

  setUp(() async {
    server = (await HttpServer.bind(InternetAddress.loopbackIPv4, 0))
      ..listen((request) async {
        final segments = request.uri.pathSegments;
        if (request.uri.path == '/health/') {
          request.response.write('ok');
        } else if (request.method == 'GET') {
          request.response
              .write('${segments.last}:${request.uri.queryParameters['q']}');
        } else if (request.method == 'POST') {
          final json = jsonDecode(await utf8.decoder.bind(request).join())
              as Map<String, dynamic>;
          request.response.write(json['title']);
        } else if (request.method == 'DELETE') {
          request.response.write(segments.last);
        }
        await request.response.close();
      });
    client = TodosClient(
      Dio(BaseOptions(baseUrl: 'http://${server.address.host}:${server.port}')),
    );
  });

  tearDown(() => server.close(force: true));

  test('GET preserves path and query parameters', () async {
    expect(await client.read('42', query: 'open'), '42:open');
  });

  test('POST serializes the declared JSON body', () async {
    expect(await client.create({'title': 'ship'}), 'ship');
  });

  test('DELETE preserves the path parameter', () async {
    expect(await client.delete('42'), '42');
  });

  test('top-level route modules generate a separate client', () async {
    final routes = HealthRoutesClient(
      Dio(BaseOptions(baseUrl: 'http://${server.address.host}:${server.port}')),
    );
    expect(await routes.health(), 'ok');
  });
}
