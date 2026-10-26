import 'dart:convert';
import 'dart:io' show gzip;

import 'package:dust_server/server.dart';
import 'package:test/test.dart';

import '../support.dart';

void main() {
  test('compression inside a router encodes a returned JSON list', () async {
    final app = Router()
      ..layer(const Compression(minimumBytes: 4))
      ..route('/todos', get((request) async => List.filled(200, 'todo')));

    final response = await app.handler(
      request('GET', '/todos', headers: const {'accept-encoding': 'gzip'}),
    );
    final bytes = await response.read().expand((chunk) => chunk).toList();

    expect(response.headers['content-encoding'], 'gzip');
    expect(jsonDecode(utf8.decode(gzip.decode(bytes))), hasLength(200));
  });
}
