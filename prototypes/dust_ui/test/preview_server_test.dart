import 'dart:convert';
import 'dart:io';

import 'package:dust_ui_prototype/src/dev/preview_server.dart';
import 'package:test/test.dart';

void main() {
  late Directory directory;
  late HttpServer server;
  late HttpClient client;
  late Uri origin;

  setUp(() async {
    directory = await Directory.systemTemp.createTemp('dust-ui-preview');
    final bundle = await Directory.fromUri(
      directory.uri.resolve('bundle/'),
    ).create();
    await File.fromUri(
      bundle.uri.resolve('index.html'),
    ).writeAsString('<title>Lab</title>');
    final source = await File.fromUri(
      directory.uri.resolve('custom.dustui'),
    ).writeAsString('unclosed {');
    await Link.fromUri(bundle.uri.resolve('outside.txt')).create(source.path);
    server = await startPreviewServer(
      bundle: bundle,
      demo: Directory('demo'),
      port: 0,
      source: source,
    );
    origin = Uri.parse('http://127.0.0.1:${server.port}');
    client = HttpClient();
  });

  tearDown(() async {
    client.close(force: true);
    await server.close(force: true);
    await directory.delete(recursive: true);
  });

  test(
    'serves the lab and the editable file even when its syntax is invalid',
    () async {
      final page = await (await client.getUrl(origin)).close();
      expect(page.statusCode, HttpStatus.ok);
      expect(page.headers.contentType?.mimeType, 'text/html');
      expect(await utf8.decoder.bind(page).join(), '<title>Lab</title>');

      final response = await (await client.getUrl(
        origin.resolve('/ui/product'),
      )).close();
      expect(response.statusCode, HttpStatus.ok);
      expect(response.headers.contentType?.mimeType, 'text/plain');
      expect(await utf8.decoder.bind(response).join(), 'unclosed {');
    },
  );

  test(
    'rejects cross-origin requests and symlinks outside the bundle',
    () async {
      final foreign = await client.getUrl(origin.resolve('/ui/product'));
      foreign.headers.set('Origin', 'https://other.example');
      final rejected = await foreign.close();
      expect(rejected.statusCode, HttpStatus.forbidden);
      await rejected.drain<void>();

      final file = await (await client.getUrl(
        origin.resolve('/outside.txt'),
      )).close();
      expect(file.statusCode, HttpStatus.notFound);
      await file.drain<void>();
    },
  );
}
