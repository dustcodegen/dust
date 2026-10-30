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
        } else if (request.uri.path.startsWith('/operations/')) {
          request.response.write(segments.last);
        } else if (request.method == 'PUT') {
          request.response.write(
            '${segments.last}:${request.uri.queryParameters['status']}:'
            '${request.uri.queryParametersAll['tag']!.join(',')}:'
            '${request.headers.value('x-token')}:'
            '${request.headers.value('x-trace')}',
          );
        } else if (request.uri.path == '/transport/search') {
          request.response.write(
            '${request.uri.queryParameters['owner']}:'
            '${request.uri.queryParameters['done']}:'
            '${request.uri.queryParameters['page']}:'
            '${request.uri.queryParametersAll['tags']!.join(',')}:'
            '${request.uri.queryParameters['order']}',
          );
        } else if (request.uri.path == '/transport/typed' &&
            request.method == 'POST') {
          final body = jsonDecode(await utf8.decoder.bind(request).join())
              as Map<String, dynamic>;
          request.response
            ..statusCode = HttpStatus.created
            ..headers.contentType = ContentType.json
            ..write(
              jsonEncode({
                'id': 'created',
                'display_name': body['display_name'],
                'order': body['order'],
                'metadata': body['metadata'],
                'tags': body['tags'],
              }),
            );
        } else if (request.uri.path == '/transport/typed') {
          request.response
            ..headers.contentType = ContentType.json
            ..write(
              jsonEncode([
                {
                  'id': 'one',
                  'display_name': 'First',
                  'order': 'oldest',
                  'metadata': {'source': 'server'},
                  'tags': <String>[],
                },
              ]),
            );
        } else if (request.uri.path == '/transport/typed/missing') {
          request.response
            ..headers.contentType = ContentType.json
            ..write('null');
        } else if (request.uri.path == '/transport/typed/malformed') {
          request.response
            ..headers.contentType = ContentType.json
            ..write(jsonEncode({'id': 7}));
        } else if (request.uri.path == '/transport/form') {
          request.response.write(await utf8.decoder.bind(request).join());
        } else if (request.uri.path == '/transport/multipart') {
          request.response.write(await utf8.decoder.bind(request).join());
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
    expect(await client.read('42'), '42:null');
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

  test('PUT merges caller query and header maps', () async {
    final transport = TransportClient(
      Dio(BaseOptions(baseUrl: 'http://${server.address.host}:${server.port}')),
    );

    expect(
      await transport.replace(
        'a/b',
        {'status': 'open'},
        ['one', 'two'],
        'secret',
        {'x-trace': 'trace-1'},
      ),
      'a/b:open:one,two:secret:trace-1',
    );
  });

  test('typed query models use canonical Serde values', () async {
    final transport = TransportClient(
      Dio(BaseOptions(baseUrl: 'http://${server.address.host}:${server.port}')),
    );

    expect(
      await transport.search(
        const TransportFilter(
          owner: 'dust',
          completed: true,
          page: 2,
          tags: ['rust', 'dart'],
          order: TransportOrder.oldest,
        ),
      ),
      'dust:true:2:rust,dart:oldest',
    );
  });

  test(
    'typed payloads encode and decode through shared Serde models',
    () async {
      final transport = TransportClient(
        Dio(
          BaseOptions(
            baseUrl: 'http://${server.address.host}:${server.port}',
          ),
        ),
      );

      final created = await transport.createTyped(
        const CreateTransport(
          title: 'Ship Dust',
          order: TransportOrder.newest,
          metadata: TransportMetadata(source: 'client'),
          tags: ['dart', 'rust'],
        ),
      );
      final listed = await transport.listTyped();

      expect(created.id, 'created');
      expect(created.title, 'Ship Dust');
      expect(created.order, TransportOrder.newest);
      expect(created.metadata.source, 'client');
      expect(created.tags, ['dart', 'rust']);
      expect(listed.single.title, 'First');
      expect(listed.single.order, TransportOrder.oldest);
    },
  );

  test(
    'typed payload decoding preserves null and rejects malformed JSON',
    () async {
      final transport = TransportClient(
        Dio(
          BaseOptions(
            baseUrl: 'http://${server.address.host}:${server.port}',
          ),
        ),
      );

      expect(await transport.readTyped('missing'), isNull);
      await expectLater(
        transport.readTyped('malformed'),
        throwsA(isA<Object>()),
      );
    },
  );

  test('PATCH encodes every declared form field', () async {
    final transport = TransportClient(
      Dio(BaseOptions(baseUrl: 'http://${server.address.host}:${server.port}')),
    );

    final body = await transport.form('Ship Dust', 'high');
    expect(body, contains('title=Ship+Dust'));
    expect(body, contains('priority=high'));
  });

  test('multipart generation preserves field names and values', () async {
    final transport = TransportClient(
      Dio(BaseOptions(baseUrl: 'http://${server.address.host}:${server.port}')),
    );

    final body = await transport.multipart('Ship Dust', 'edge');
    expect(body, contains('name="title"'));
    expect(body, contains('Ship Dust'));
    expect(body, contains('name="note"'));
    expect(body, contains('edge'));
  });

  test('HEAD and OPTIONS generate body-free calls', () async {
    final transport = TransportClient(
      Dio(BaseOptions(baseUrl: 'http://${server.address.host}:${server.port}')),
    );

    await expectLater(transport.head('42'), completes);
    await expectLater(transport.options(), completes);
  });

  test('route tags expose public and shared operations', () async {
    final operations = OperationsClient(
      Dio(BaseOptions(baseUrl: 'http://${server.address.host}:${server.port}')),
    );

    expect(await operations.publicOnly(), 'public');
    expect(await operations.shared(), 'shared');
  });
}
