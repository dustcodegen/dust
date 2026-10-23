import 'dart:convert';

import 'package:dust_server/server.dart';
import 'package:server_codegen_app/server_codegen_app.dart';
import 'package:test/test.dart';

void main() {
  late Handler handler;

  setUp(() {
    handler = (Router()
          ..merge(TodoController().routes())
          ..merge(todoControllerRoutes()))
        .handler;
  });

  test('generated top-level route is mountable', () async {
    final response = await handler(
      Request('GET', Uri.parse('http://localhost/health')),
    );

    expect(response.statusCode, 200);
    expect(jsonDecode(await response.readAsString()), {'status': 'ok'});
  });

  test('generated GET extracts path, query, and header values', () async {
    final response = await handler(
      Request(
        'GET',
        Uri.parse('http://localhost/todos/7?done=true'),
        headers: {'x-trace-id': 'trace-1'},
      ),
    );

    expect(response.statusCode, 200);
    expect(jsonDecode(await response.readAsString()), {
      'id': 7,
      'done': true,
      'traceId': 'trace-1',
    });
  });

  test('generated POST decodes JSON and applies its success status', () async {
    final response = await handler(
      Request(
        'POST',
        Uri.parse('http://localhost/todos'),
        headers: {'content-type': 'application/json'},
        body: jsonEncode({'title': 'ship Dust'}),
      ),
    );

    expect(response.statusCode, 201);
    expect(jsonDecode(await response.readAsString()), {'title': 'ship Dust'});
  });

  test('generated void result becomes no content', () async {
    final response = await handler(
      Request('DELETE', Uri.parse('http://localhost/todos')),
    );

    expect(response.statusCode, 204);
    expect(await response.readAsString(), isEmpty);
  });

  test('generated Response result remains unchanged', () async {
    final response = await handler(
      Request('OPTIONS', Uri.parse('http://localhost/todos/raw')),
    );

    expect(response.statusCode, 202);
    expect(await response.readAsString(), 'raw');
  });

  test('generated path extractor returns a typed rejection', () async {
    final response = await handler(
      Request('GET', Uri.parse('http://localhost/todos/not-an-int')),
    );

    expect(response.statusCode, 400);
    expect(
      jsonDecode(await response.readAsString()),
      containsPair('error', contains('path parameter "id"')),
    );
  });
}
