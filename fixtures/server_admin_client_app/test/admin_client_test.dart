import 'dart:io';

import 'package:dio/dio.dart';
import 'package:server_admin_client_app/server_admin_client_app.dart';
import 'package:test/test.dart';

void main() {
  test('admin tag generates only the administrative client', () async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    server.listen((request) async {
      request.response.write('admin');
      await request.response.close();
    });
    addTearDown(() => server.close(force: true));
    final client = AdminTodosClient(
      Dio(BaseOptions(baseUrl: 'http://${server.address.host}:${server.port}')),
    );

    expect(await client.list(), 'admin');
  });
}
