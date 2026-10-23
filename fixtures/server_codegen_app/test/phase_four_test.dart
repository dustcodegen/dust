import 'dart:convert';
import 'dart:io';

import 'package:dust_server/server.dart';
import 'package:server_codegen_app/server_codegen_app.dart';
import 'package:test/test.dart';

void main() {
  late ServerHandle server;
  late HttpClient client;
  late List<Object> errors;

  setUp(() async {
    errors = [];
    client = HttpClient();
    server = await startServer(
      address: InternetAddress.loopbackIPv4,
      port: 0,
      onError: (error, _) => errors.add(error),
    );
  });

  tearDown(() async {
    client.close(force: true);
    expect(await server.close(drain: const Duration(seconds: 1)), isTrue);
  });

  Uri uri(String path) => Uri.http('127.0.0.1:${server.port}', path);

  Future<HttpClientResponse> send(
    String method,
    String path, {
    String? body,
  }) async {
    final request = await client.openUrl(method, uri(path));
    if (body != null) request.write(body);
    return request.close();
  }

  test('real server composes generated groups and normalized prefixes',
      () async {
    final response = await send('GET', '/api/v1/health');

    expect(response.statusCode, 200);
    expect(jsonDecode(await utf8.decodeStream(response)), {'status': 'ok'});
  });

  test('layers run root, controller, then route', () async {
    final response = await send('GET', '/api/v1/todos/layers');

    expect(response.statusCode, 200);
    expect(await utf8.decodeStream(response), 'root,controller,route');
  });

  test('root body limit reaches generated body extraction', () async {
    final response = await send(
      'POST',
      '/api/v1/todos/raw-text',
      body: List.filled(65, 'x').join(),
    );

    expect(response.statusCode, 413);
  });

  test('request-scoped error sink sees opaque generated failures', () async {
    final response = await send('GET', '/api/v1/todos/failure');

    expect(response.statusCode, 500);
    expect(errors, [isA<StateError>()]);
  });
}
