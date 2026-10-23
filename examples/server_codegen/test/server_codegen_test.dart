import 'package:http/http.dart' as http;
import 'package:server_codegen/bearer_auth.dart' as bearer_auth;
import 'package:server_codegen/controller_state.dart' as controller_state;
import 'package:server_codegen/hello_world.dart' as hello_world;
import 'package:server_codegen/json_body.dart' as json_body;
import 'package:server_codegen/multipart_form.dart' as multipart_form;
import 'package:server_codegen/path_and_query.dart' as path_and_query;
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

  test('generated controller reads typed router state', () async {
    final app = await serveExample(controller_state.buildApp());

    final response = await app.send('GET', '/greetings/Ada');

    expect(app.json(response), {'message': 'Hello, Ada!'});
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
}
