import 'dart:convert';

import 'package:dust_server/server.dart';
import 'package:server_codegen_app/server_codegen_app.dart';
import 'package:test/test.dart';

void main() {
  late Handler handler;

  setUp(() {
    handler = (Router(bodyLimit: 512)
          ..merge(TodoController().routes())
          ..withState(const TodoState('fixture')))
        .handler;
  });

  test('generated form handlers share one body and validate models', () async {
    final fields = await handler(
      Request(
        'POST',
        Uri.parse('http://localhost/todos/form-fields'),
        headers: {'content-type': 'application/x-www-form-urlencoded'},
        body: 'title=ship+Dust',
      ),
    );
    final model = await handler(
      Request(
        'POST',
        Uri.parse('http://localhost/todos/form-model'),
        headers: {'content-type': 'application/x-www-form-urlencoded'},
        body: 'title=ship+Dust',
      ),
    );
    final invalid = await handler(
      Request(
        'POST',
        Uri.parse('http://localhost/todos/form-model'),
        headers: {'content-type': 'application/x-www-form-urlencoded'},
        body: 'title=',
      ),
    );

    expect(jsonDecode(await fields.readAsString()), {
      'title': 'ship Dust',
      'priority': false,
    });
    expect(jsonDecode(await model.readAsString()), {'title': 'ship Dust'});
    expect(invalid.statusCode, 422);
  });

  test('generated multipart handler shares one body across parts', () async {
    const boundary = 'dust-boundary';
    const body = '--$boundary\r\n'
        'Content-Disposition: form-data; name="title"\r\n\r\n'
        'ship Dust\r\n'
        '--$boundary\r\n'
        'Content-Disposition: form-data; name="file"; filename="todo.txt"\r\n'
        'Content-Type: text/plain\r\n\r\n'
        'abc\r\n'
        '--$boundary--\r\n';
    final response = await handler(
      Request(
        'POST',
        Uri.parse('http://localhost/todos/multipart'),
        headers: {'content-type': 'multipart/form-data; boundary=$boundary'},
        body: body,
      ),
    );

    expect(response.statusCode, 200);
    expect(jsonDecode(await response.readAsString()), {
      'title': 'ship Dust',
      'filename': 'todo.txt',
      'size': 3,
    });
  });

  test('generated raw handlers preserve text, bytes, and streams', () async {
    final text = await handler(
      Request(
        'POST',
        Uri.parse('http://localhost/todos/raw-text'),
        body: 'hello',
      ),
    );
    final bytes = await handler(
      Request(
        'POST',
        Uri.parse('http://localhost/todos/raw-bytes'),
        body: [104, 105],
      ),
    );
    final stream = await handler(
      Request(
        'POST',
        Uri.parse('http://localhost/todos/raw-stream'),
        body: [111, 107],
      ),
    );

    expect(await text.readAsString(), 'hello');
    expect(bytes.statusCode, 202);
    expect(bytes.headers['content-type'], 'application/octet-stream');
    expect(await bytes.readAsString(), 'hi');
    expect(stream.headers['content-type'], 'application/octet-stream');
    expect(await stream.readAsString(), 'ok');
  });

  test('generated context extractors and request passthrough work', () async {
    final context = await handler(
      Request(
        'GET',
        Uri.parse('http://localhost/todos/context?draft=true'),
        context: const {
          PeerExtractable.contextKey: PeerInfo(
            remoteAddress: '127.0.0.1',
            remotePort: 4040,
            localPort: 8080,
          ),
        },
      ),
    );
    final request = await handler(
      Request('GET', Uri.parse('http://localhost/todos/request')),
    );

    expect(jsonDecode(await context.readAsString()), {
      'raw': 'draft=true',
      'state': 'fixture',
      'peer': '127.0.0.1',
      'method': 'GET',
    });
    expect(await request.readAsString(), 'GET');
  });

  test('generated response dispatch covers Result, Option, Unit, and guard',
      () async {
    final result = await handler(
      Request('GET', Uri.parse('http://localhost/todos/result')),
    );
    final option = await handler(
      Request('GET', Uri.parse('http://localhost/todos/option')),
    );
    final unit = await handler(
      Request('POST', Uri.parse('http://localhost/todos/unit')),
    );
    final failure = await handler(
      Request('GET', Uri.parse('http://localhost/todos/failure')),
    );

    expect(await result.readAsString(), 'ok');
    expect(option.statusCode, 404);
    expect(unit.statusCode, 204);
    expect(failure.statusCode, 500);
  });

  test('generated handlers preserve the rejection taxonomy', () async {
    Future<Response> create(String body, {String? contentType}) async {
      return handler(
        Request(
          'POST',
          Uri.parse('http://localhost/todos'),
          headers: {if (contentType != null) 'content-type': contentType},
          body: body,
        ),
      );
    }

    expect((await create('{"title":"ok"}')).statusCode, 415);
    expect(
      (await create('{', contentType: 'application/json')).statusCode,
      400,
    );
    expect(
      (await create('{"title":1}', contentType: 'application/json')).statusCode,
      422,
    );
    expect(
      (await create('{"title":""}', contentType: 'application/json'))
          .statusCode,
      422,
    );
    expect(
      (await handler(
        Request('GET', Uri.parse('http://localhost/todos/query')),
      ))
          .statusCode,
      400,
    );
    expect(
      (await handler(
        Request(
          'POST',
          Uri.parse('http://localhost/todos/raw-bytes'),
          body: List<int>.filled(513, 1),
        ),
      ))
          .statusCode,
      413,
    );
  });
}
