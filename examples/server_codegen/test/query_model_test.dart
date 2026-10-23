import 'package:server_codegen/path_and_query.dart' as path_and_query;
import 'package:test/test.dart';

import 'support.dart';

void main() {
  test('generated query models reuse SerDe and validation', () async {
    final app = await serveExample(path_and_query.buildApp());

    final valid = await app.send(
      'GET',
      '/todos/search?owner=ada&done=true&page=2&tags=server&tags=dart&labels=ignored&order=oldest',
    );
    final defaults = await app.send('GET', '/todos/search?owner=ada');
    final alias = await app.send(
      'GET',
      '/todos/search?owner=ada&completed=false',
    );
    final primaryWins = await app.send(
      'GET',
      '/todos/search?owner=ada&done=true&completed=not-a-boolean',
    );
    final repeatedAlias = await app.send(
      'GET',
      '/todos/search?owner=ada&labels=server&labels=dart',
    );
    final missing = await app.send('GET', '/todos/search');
    final malformed = await app.send(
      'GET',
      '/todos/search?owner=ada&page=many',
    );
    final invalid = await app.send(
      'GET',
      '/todos/search?owner=ada&page=0',
    );
    final unknown = await app.send(
      'GET',
      '/todos/search?owner=ada&extra=value',
    );

    expect(app.json(valid), {
      'owner': 'ada',
      'completed': true,
      'page': 2,
      'tags': ['server', 'dart'],
      'order': 'oldest',
    });
    expect(app.json(defaults), {
      'owner': 'ada',
      'completed': null,
      'page': 1,
      'tags': <Object?>[],
      'order': 'recent',
    });
    expect(app.json(alias)['completed'], isFalse);
    expect(primaryWins.statusCode, 200);
    expect(app.json(primaryWins)['completed'], isTrue);
    expect(app.json(repeatedAlias)['tags'], ['server', 'dart']);
    expect(missing.statusCode, 400);
    expect(app.json(missing)['error'], contains('owner'));
    expect(malformed.statusCode, 400);
    expect(invalid.statusCode, 422);
    expect(unknown.statusCode, 400);
  });
}
