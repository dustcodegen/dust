import 'package:dust_server/server.dart' show AccessRecord;
import 'package:http/http.dart' as http;
import 'package:server_codegen/bearer_auth.dart' as bearer_auth;
import 'package:server_codegen/controller_state.dart' as controller_state;
import 'package:server_codegen/forms_and_raw.dart' as forms_and_raw;
import 'package:server_codegen/hello_world.dart' as hello_world;
import 'package:server_codegen/json_body.dart' as json_body;
import 'package:server_codegen/multipart_form.dart' as multipart_form;
import 'package:server_codegen/path_and_query.dart' as path_and_query;
import 'package:server_codegen/production_app.dart' as production_app;
import 'package:server_codegen/request_data.dart' as request_data;
import 'package:server_codegen/verbs_and_responses.dart' as verbs_and_responses;
import 'package:test/test.dart';

import 'support.dart';

void main() {
  test('generated hello-world routes return text and JSON', () async {
    final app = await serveExample(hello_world.buildApp());

    final text = await app.send('GET', '/');
    final json = await app.send('GET', '/json');

    expect(text.statusCode, 200);
    expect(text.body, 'Hello, world!');
    expect(app.json(json), {'greeting': 'Hello, world!'});
  });

  test('generated path and query values are typed', () async {
    final app = await serveExample(path_and_query.buildApp());

    final valid = await app.send('GET', '/todos/7?completed=true');
    final invalid = await app.send('GET', '/todos/not-an-int');

    expect(app.json(valid), {'id': 7, 'completed': true});
    expect(invalid.statusCode, 400);
  });

  test('generated JSON body is decoded and validated', () async {
    final app = await serveExample(json_body.buildApp());

    final created = await app.send('POST', '/notes', body: {'title': 'Ship'});
    final invalid = await app.send('POST', '/notes', body: {'title': ''});

    expect(created.statusCode, 201);
    expect(app.json(created), {'title': 'Ship'});
    expect(invalid.statusCode, 422);
  });

  test('generated JSON body preserves the rejection contract', () async {
    final app = await serveExample(json_body.buildApp());

    final wrongMedia = await app.send(
      'POST',
      '/notes',
      body: '{}',
      headers: {'content-type': 'text/plain'},
    );
    final malformed = await app.send('POST', '/notes', body: '{');
    final wrongShape = await app.send('POST', '/notes', body: {'title': 1});

    expect(wrongMedia.statusCode, 415);
    expect(malformed.statusCode, 400);
    expect(wrongShape.statusCode, 422);
  });

  test('generated controller reads typed router state', () async {
    final app = await serveExample(controller_state.buildApp());

    final response = await app.send('GET', '/greetings/Ada');

    expect(app.json(response), {'message': 'Hello, Ada!'});
    expect(response.headers['x-content-type-options'], 'nosniff');
  });

  test('generated request extractors expose broad request data', () async {
    final app = await serveExample(request_data.buildApp());

    final response = await app.send(
      'GET',
      '/request-data/inspect?tag=red&tag=blue&mode=full',
      headers: {'x-request-id': 'demo-1'},
    );
    final body = app.json(response);

    expect(body['tags'], ['red', 'blue']);
    expect(body['query'], {'tag': 'blue', 'mode': 'full'});
    expect(body['rawQuery'], 'tag=red&tag=blue&mode=full');
    expect(body['requestId'], 'demo-1');
    expect(body['host'], isNotEmpty);
    expect(body['peer'], '127.0.0.1');
    expect(body['method'], 'GET');
    expect(body['cancelled'], isFalse);
  });

  test('generated form fields share one body read', () async {
    final app = await serveExample(forms_and_raw.buildApp());

    final response = await app.send(
      'POST',
      '/bodies/form',
      body: 'title=Ship+Dust&priority=2',
      headers: {'content-type': 'application/x-www-form-urlencoded'},
    );

    expect(app.json(response), {'title': 'Ship Dust', 'priority': 2});
  });

  test('generated raw body handlers preserve text bytes and streams', () async {
    final app = await serveExample(forms_and_raw.buildApp());

    final text = await app.send('POST', '/bodies/text', body: 'plain text');
    final bytes = await app.send('POST', '/bodies/bytes', body: [1, 2, 3]);
    final stream = await app.send('POST', '/bodies/stream', body: 'streamed');

    expect(text.body, 'plain text');
    expect(bytes.statusCode, 202);
    expect(bytes.bodyBytes, [1, 2, 3]);
    expect(stream.body, 'streamed');
  });

  test('generated routes support all supported HTTP verbs', () async {
    final app = await serveExample(verbs_and_responses.buildApp());

    final put = await app.send('PUT', '/resources/7');
    final patch = await app.send('PATCH', '/resources/7');
    final delete = await app.send('DELETE', '/resources/7');
    final head = await app.send('HEAD', '/resources/7');
    final options = await app.send('OPTIONS', '/resources');

    expect(app.json(put), {'method': 'PUT', 'id': 7});
    expect(app.json(patch), {'method': 'PATCH', 'id': 7});
    expect(delete.statusCode, 204);
    expect(head.statusCode, 200);
    expect(head.body, isEmpty);
    expect(options.statusCode, 204);
  });

  test('generated response dispatch preserves wrappers and responses',
      () async {
    final app = await serveExample(verbs_and_responses.buildApp());

    final result = await app.send('GET', '/resources/result');
    final missing = await app.send('GET', '/resources/missing');
    final custom = await app.send('GET', '/resources/custom');

    expect(result.body, 'ok');
    expect(missing.statusCode, 404);
    expect(custom.statusCode, 202);
    expect(custom.body, 'custom');
  });

  test('generated custom extractor enforces bearer auth', () async {
    final app = await serveExample(bearer_auth.buildApp());

    final accepted = await app.send(
      'GET',
      '/me',
      headers: {'authorization': 'Bearer demo-token'},
    );
    final missing = await app.send('GET', '/me');
    final forbidden = await app.send(
      'GET',
      '/me',
      headers: {'authorization': 'Bearer wrong'},
    );

    expect(app.json(accepted), {'user': 'Ada'});
    expect(missing.statusCode, 401);
    expect(forbidden.statusCode, 403);
  });

  test('generated multipart handler reads fields and files', () async {
    final app = await serveExample(multipart_form.buildApp());
    final uri = Uri.parse(
      'http://${app.server.address.host}:${app.server.port}/upload',
    );
    final request = http.MultipartRequest('POST', uri)
      ..fields['caption'] = 'notes'
      ..files.add(
        http.MultipartFile.fromString('file', 'Dust', filename: 'dust.txt'),
      );

    final response = await http.Response.fromStream(await request.send());

    expect(response.statusCode, 200);
    expect(app.json(response), {
      'caption': 'notes',
      'filename': 'dust.txt',
      'bytes': 4,
    });
  });

  test('multipart handler rejects a missing file', () async {
    final app = await serveExample(multipart_form.buildApp());
    final uri = Uri.parse(
      'http://${app.server.address.host}:${app.server.port}/upload',
    );
    final request = http.MultipartRequest('POST', uri)
      ..fields['caption'] = 'notes';

    final response = await http.Response.fromStream(await request.send());

    expect(response.statusCode, greaterThanOrEqualTo(400));
    expect(response.statusCode, lessThan(500));
  });

  test('multipart handler rejects invalid UTF-8 text', () async {
    final app = await serveExample(multipart_form.buildApp());
    final uri = Uri.parse(
      'http://${app.server.address.host}:${app.server.port}/upload',
    );
    final request = http.MultipartRequest('POST', uri)
      ..files.add(http.MultipartFile.fromBytes('caption', [0xff]))
      ..files.add(
        http.MultipartFile.fromString('file', 'Dust', filename: 'dust.txt'),
      );

    final response = await http.Response.fromStream(await request.send());

    expect(response.statusCode, 400);
  });

  test('production composition applies limits and operational layers',
      () async {
    final records = <AccessRecord>[];
    final app = await serveExample(
      production_app.buildApp(onRecord: records.add),
    );

    final created = await app.send(
      'POST',
      '/notes',
      body: {'title': 'Ship'},
      headers: {'x-request-id': 'example-request'},
    );
    final tooLarge = await app.send(
      'POST',
      '/notes',
      body: List<int>.filled(64 * 1024 + 1, 0),
      headers: {'content-type': 'application/json'},
    );

    expect(created.statusCode, 201);
    expect(created.headers['x-request-id'], 'example-request');
    expect(created.headers['x-content-type-options'], 'nosniff');
    expect(tooLarge.statusCode, 413);
    expect(records.map((record) => record.status), [201, 413]);
  });
}
