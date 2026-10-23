import 'dart:convert';

import 'package:dust_server/server.dart';
import 'package:server_codegen_app/server_codegen_app.dart';
import 'package:test/test.dart';

void main() {
  final handler = (Router()..merge(basicRoutes())).handler;

  test('function GET extracts path and query parameters', () async {
    final response = await handler(
      Request(
        'GET',
        Uri.parse('http://localhost/items/7?details=true'),
      ),
    );

    expect(response.statusCode, 200);
    expect(jsonDecode(await response.readAsString()), {
      'id': 7,
      'details': true,
    });
  });

  test('function POST extracts its required query parameter', () async {
    final response = await handler(
      Request(
        'POST',
        Uri.parse('http://localhost/items?name=Dust'),
      ),
    );

    expect(response.statusCode, 201);
    expect(jsonDecode(await response.readAsString()), {'name': 'Dust'});
  });

  test('function DELETE extracts a path parameter and returns no content',
      () async {
    final response = await handler(
      Request('DELETE', Uri.parse('http://localhost/items/7')),
    );

    expect(response.statusCode, 204);
    expect(await response.readAsString(), isEmpty);
  });

  test('function parameters reject missing or invalid input', () async {
    final invalidPath = await handler(
      Request('GET', Uri.parse('http://localhost/items/not-an-int')),
    );
    final missingQuery = await handler(
      Request('POST', Uri.parse('http://localhost/items')),
    );

    expect(invalidPath.statusCode, 400);
    expect(missingQuery.statusCode, 400);
  });
}
