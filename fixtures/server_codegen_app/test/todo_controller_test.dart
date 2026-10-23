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

  test('generated custom extractor short-circuits on rejection', () async {
    final rejected = await handler(
      Request('GET', Uri.parse('http://localhost/todos/required-auth')),
    );
    final accepted = await handler(
      Request(
        'GET',
        Uri.parse('http://localhost/todos/required-auth'),
        headers: {'authorization': 'Bearer user-1'},
      ),
    );

    expect(rejected.statusCode, 401);
    expect(accepted.statusCode, 200);
    expect(jsonDecode(await accepted.readAsString()), {'user': 'user-1'});
  });

  test('generated Option wrapper collapses client rejection', () async {
    final anonymous = await handler(
      Request('GET', Uri.parse('http://localhost/todos/optional-auth')),
    );
    final authenticated = await handler(
      Request(
        'GET',
        Uri.parse('http://localhost/todos/optional-auth'),
        headers: {'authorization': 'Bearer user-1'},
      ),
    );

    expect(jsonDecode(await anonymous.readAsString()), {
      'authenticated': false,
    });
    expect(jsonDecode(await authenticated.readAsString()), {
      'authenticated': true,
    });
  });

  test('generated Option wrapper also wraps a built-in extractor', () async {
    final missing = await handler(
      Request('GET', Uri.parse('http://localhost/todos/optional-page')),
    );
    final present = await handler(
      Request(
        'GET',
        Uri.parse('http://localhost/todos/optional-page?page=2'),
      ),
    );

    expect(jsonDecode(await missing.readAsString()), {'present': false});
    expect(jsonDecode(await present.readAsString()), {'present': true});
  });

  test('generated Result wrapper passes scoped rejection to handler', () async {
    final denied = await handler(
      Request(
        'GET',
        Uri.parse('http://localhost/todos/fallible-write-auth'),
        headers: {'authorization': 'Bearer user-1:todos:read'},
      ),
    );
    final allowed = await handler(
      Request(
        'GET',
        Uri.parse('http://localhost/todos/fallible-write-auth'),
        headers: {'authorization': 'Bearer user-1:todos:write'},
      ),
    );

    expect(jsonDecode(await denied.readAsString()), {'authorized': false});
    expect(jsonDecode(await allowed.readAsString()), {'authorized': true});
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
