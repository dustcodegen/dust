import 'dart:convert';
import 'dart:io';

import 'package:dust_ui_prototype/language.dart';
import 'package:dust_ui_prototype/src/dev/demo_routes.dart';
import 'package:flutter_test/flutter_test.dart';

typedef _Response = ({
  int status,
  String source,
  String? contentType,
  String? cacheControl,
  String? allow,
});

void main() {
  late HttpServer server;
  late HttpClient client;

  setUp(() async {
    server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    final demo = DemoRoutes(directory: Directory('demo'));
    server.listen((request) async {
      if (!await demo.handle(request)) {
        request.response.statusCode = HttpStatus.notImplemented;
        await request.response.close();
      }
    });
    client = HttpClient();
  });

  tearDown(() async {
    client.close(force: true);
    await server.close(force: true);
  });

  Future<_Response> send(
    String method,
    String path, {
    String? body,
    ContentType? contentType,
  }) async {
    final uri = Uri.http('127.0.0.1:${server.port}', path);
    final request = await client.openUrl(method, uri);
    if (body != null) {
      request.headers.contentType = contentType ?? ContentType.json;
      request.write(body);
    }
    final response = await request.close();
    return (
      status: response.statusCode,
      source: await utf8.decoder.bind(response).join(),
      contentType: response.headers.contentType?.mimeType,
      cacheControl: response.headers.value(HttpHeaders.cacheControlHeader),
      allow: response.headers.value(HttpHeaders.allowHeader),
    );
  }

  test('serves a parseable product and empty cart', () async {
    final product = await send('GET', '/ui/product');
    expect(product.status, HttpStatus.ok);
    expect(product.contentType, 'text/plain');
    expect(product.cacheControl, 'no-store');
    expect(parseUi(product.source).props['id'], 'product');

    final summary = await send('GET', '/ui/cart/summary');
    final root = parseUi(summary.source);
    expect(root.props['id'], 'cart');
    expect(root.children.last.props['value'], 'Your cart is empty.');
  });

  test(
    'adds a valid item and escapes its note in the returned source',
    () async {
      const note = 'Gift "wrap"\n} text(value: "Injected")';
      final updated = await send(
        'POST',
        '/ui/cart/items',
        body: jsonEncode({
          'productId': 'linen-shirt-01',
          'size': 'M',
          'quantity': 2,
          'note': note,
        }),
      );
      expect(updated.status, HttpStatus.ok);
      final region = parseUi(updated.source);
      expect(region.props['id'], 'cart');
      expect(
        region.children[1].children.first.props['value'],
        'Everyday linen shirt × 2 · Size M',
      );
      expect(region.children[2].props['value'], 'Note: $note');
      expect(region.children.last.props['value'], r'Total: $78.00');

      final modal = await send('GET', '/ui/cart');
      final content = parseUi(modal.source);
      expect(content.children.first.props['value'], 'Your cart');
      expect(content.children.last.props['value'], r'Total: $78.00');
    },
  );

  test(
    'rejects invalid and oversized requests without changing cart',
    () async {
      final badSize = await send(
        'POST',
        '/ui/cart/items',
        body: '{"productId":"linen-shirt-01","size":"XXL"}',
      );
      expect(badSize.status, HttpStatus.badRequest);

      final wrongType = await send(
        'POST',
        '/ui/cart/items',
        contentType: ContentType.text,
        body: '{"productId":"linen-shirt-01","size":"M"}',
      );
      expect(wrongType.status, HttpStatus.unsupportedMediaType);

      final huge = await send('POST', '/ui/cart/items', body: 'x' * 4097);
      expect(huge.status, HttpStatus.requestEntityTooLarge);

      final summary = await send('GET', '/ui/cart/summary');
      expect(
        parseUi(summary.source).children.last.props['value'],
        'Your cart is empty.',
      );
    },
  );

  test('claims only demo paths and checks request methods', () async {
    expect((await send('GET', '/')).status, HttpStatus.notImplemented);
    expect((await send('GET', '/ui/missing')).status, HttpStatus.notFound);
    final method = await send('GET', '/ui/cart/items');
    expect(method.status, HttpStatus.methodNotAllowed);
    expect(method.allow, 'POST');
  });
}
