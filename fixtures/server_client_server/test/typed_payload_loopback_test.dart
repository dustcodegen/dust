import 'dart:io';

import 'package:dio/dio.dart';
import 'package:dust_server/server.dart';
import 'package:server_client_app/query_contracts.dart';
import 'package:server_client_app/src/generated/client_edges_client.transport.g.dart';
import 'package:server_client_server/client_edges.dart';
import 'package:test/test.dart';

void main() {
  late ServerHandle server;
  late Dio dio;
  late TransportClient client;

  setUpAll(() async {
    server = await serve(
      Transport().routes(),
      InternetAddress.loopbackIPv4,
      0,
    );
    dio = Dio(
      BaseOptions(
        baseUrl: 'http://${server.address.host}:${server.port}',
      ),
    );
    client = TransportClient(dio);
  });

  tearDownAll(() async {
    dio.close(force: true);
    await server.close();
  });

  test('generated client and server share typed Serde payloads', () async {
    final created = await client.createTyped(
      const CreateTransport(
        title: 'Ship Dust',
        order: TransportOrder.newest,
        metadata: TransportMetadata(source: 'loopback'),
        tags: ['dart', 'rust'],
      ),
    );
    final listed = await client.listTyped();
    final missing = await client.readTyped('missing');

    expect(created.id, 'created');
    expect(created.title, 'Ship Dust');
    expect(created.metadata.source, 'loopback');
    expect(created.tags, ['dart', 'rust']);
    expect(listed.single.title, 'First');
    expect(missing, isNull);
  });

  test('generated server validates the imported request model', () async {
    final request = client.createTyped(
      const CreateTransport(
        title: '',
        order: TransportOrder.newest,
        metadata: TransportMetadata(source: 'loopback'),
      ),
    );

    await expectLater(
      request,
      throwsA(
        isA<DioException>().having(
          (error) => error.response?.statusCode,
          'statusCode',
          422,
        ),
      ),
    );
  });
}
