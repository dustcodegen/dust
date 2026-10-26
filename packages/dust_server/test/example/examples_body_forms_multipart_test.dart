import 'dart:convert';
import 'dart:io';
import 'package:http/http.dart' as http;
import 'package:test/test.dart';
import '../../example/multipart_form.dart' as multipart_form;
import '../../example/multipart_stream.dart' as multipart_stream;
import 'serve.dart';

/// Reading a request: paths, queries, headers, bodies and forms.
///
/// Split out of one suite per example so each file stays readable; they
/// share the harness in `support.dart`.
/// Form and multipart bodies, and dispatch by content type.
/// Multipart bodies, buffered and streamed.
void main() {
  group('multipart_form', () {
    /// Builds a multipart body by hand, so the test does not depend on a client.
    ({String body, String type}) multipart({bool withFile = true}) {
      const boundary = 'X-DUST-BOUNDARY';
      final caption = '--$boundary\r\n'
          'content-disposition: form-data; name="caption"\r\n\r\n'
          'my cat\r\n';
      final photo = '--$boundary\r\n'
          'content-disposition: form-data; name="photo"; '
          'filename="cat.txt"\r\n'
          'content-type: text/plain\r\n\r\n'
          'meow\r\n';
      final parts = <String>[
        caption,
        if (withFile) photo,
        '--$boundary--\r\n',
      ];
      return (
        body: parts.join(),
        type: 'multipart/form-data; boundary=$boundary',
      );
    }

    test('reads the file and the ordinary field beside it', () async {
      final app = await example(multipart_form.buildApp());
      final sent = multipart();

      final response = await app.send(
        'POST',
        '/upload',
        body: sent.body,
        headers: {'content-type': sent.type},
      );

      expect(response.statusCode, 200);
      expect(app.object(response), {
        'filename': 'cat.txt',
        'bytes': 4,
        'caption': 'my cat',
      });
    });

    test('a missing file part is a rejection, not a crash', () async {
      final app = await example(multipart_form.buildApp());
      final sent = multipart(withFile: false);

      final response = await app.send(
        'POST',
        '/upload',
        body: sent.body,
        headers: {'content-type': sent.type},
      );

      expect(response.statusCode, greaterThanOrEqualTo(400));
      expect(response.statusCode, lessThan(500));
    });

    test('a non-multipart body is a 415', () async {
      final app = await example(multipart_form.buildApp());

      expect((await app.post('/upload', const {})).statusCode, 415);
    });
  });

  group('multipart_stream', () {
    /// A multipart body with one field and one file of [bytes] bytes.
    ({String type, List<int> body}) upload({
      int bytes = 200000,
      List<int>? caption,
    }) {
      const boundary = 'X-DUST-STREAM';
      final out = <int>[
        ...utf8.encode('--$boundary\r\n'
            'content-disposition: form-data; name="caption"\r\n\r\n'),
        ...?caption,
        if (caption == null) ...utf8.encode('a big one'),
        ...utf8.encode('\r\n'),
        ...utf8.encode('--$boundary\r\n'
            'content-disposition: form-data; name="file"; '
            'filename="../../etc/passwd"\r\n'
            'content-type: application/octet-stream\r\n\r\n'),
        ...List.filled(bytes, 7),
        ...utf8.encode('\r\n--$boundary--\r\n'),
      ];

      return (type: 'multipart/form-data; boundary=$boundary', body: out);
    }

    Future<http.Response> sendBytes(
      ExampleApp app,
      List<int> body,
      String contentType,
    ) async {
      final request = http.Request('POST', app.uri('/upload'))
        ..headers['content-type'] = contentType
        ..bodyBytes = body;
      return http.Response.fromStream(await request.send());
    }

    Future<String> directory() async {
      final root = await Directory.systemTemp.createTemp('dust-example-up-');
      addTearDown(() => root.delete(recursive: true));
      return root.path;
    }

    test('writes the file to disk and reports what it stored', () async {
      final root = await directory();
      final app = await example(multipart_stream.buildApp(root));
      final sent = upload();

      final response = await app.send(
        'POST',
        '/upload',
        body: String.fromCharCodes(sent.body),
        headers: {'content-type': sent.type},
      );

      expect(response.statusCode, 201);
      final decoded = app.object(response);
      expect((decoded['fields']! as Map)['caption'], 'a big one');
      expect(decoded['files'], hasLength(1));
      expect((decoded['files']! as List).first, isA<Map<String, Object?>>());
    });

    test('stores under a generated id, never the client filename', () async {
      // `../../etc/passwd` is a valid filename as far as the client is
      // concerned. It is kept as data and never used as a path.
      final root = await directory();
      final app = await example(multipart_stream.buildApp(root));
      final sent = upload();

      final response = await app.send(
        'POST',
        '/upload',
        body: String.fromCharCodes(sent.body),
        headers: {'content-type': sent.type},
      );

      final file = (app.object(response)['files']! as List).first! as Map;
      expect(file['filename'], '../../etc/passwd');
      expect(file['id'], matches(r'^[a-z0-9]{16}$'));

      final written = Directory(root).listSync().whereType<File>().toList();
      expect(written, hasLength(1));
      expect(written.single.path, endsWith(file['id'] as String));
    });

    test('two ids never collide', () async {
      final root = await directory();
      final app = await example(multipart_stream.buildApp(root));

      final ids = <Object?>{};
      for (var index = 0; index < 4; index++) {
        final sent = upload(bytes: 32);
        final response = await app.send(
          'POST',
          '/upload',
          body: String.fromCharCodes(sent.body),
          headers: {'content-type': sent.type},
        );
        ids.add(((app.object(response)['files']! as List).first! as Map)['id']);
      }

      expect(ids, hasLength(4));
    });

    test('a body over the limit is refused', () async {
      final root = await directory();
      final app = await example(multipart_stream.buildApp(root));
      final sent = upload(bytes: 11 * 1024 * 1024);

      final response = await app.send(
        'POST',
        '/upload',
        body: String.fromCharCodes(sent.body),
        headers: {'content-type': sent.type},
      );

      expect(response.statusCode, 413);
    });

    test('malformed multipart syntax is a 400', () async {
      final root = await directory();
      final app = await example(multipart_stream.buildApp(root));

      final response = await app.send(
        'POST',
        '/upload',
        body: 'not multipart',
        headers: const {
          'content-type': 'multipart/form-data; boundary=X-DUST-STREAM',
        },
      );

      expect(response.statusCode, 400);
    });

    test('invalid UTF-8 text is a 400', () async {
      final root = await directory();
      final app = await example(multipart_stream.buildApp(root));
      final sent = upload(bytes: 0, caption: [0xff]);

      final response = await sendBytes(app, sent.body, sent.type);

      expect(response.statusCode, 400);
    });
  });
}
