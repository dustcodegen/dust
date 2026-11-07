import 'dart:convert';
import 'dart:io';

const _productId = 'linen-shirt-01';
const _productName = 'Everyday linen shirt';
const _price = 39;
const _maxBodyBytes = 4096;
const _maxCartLines = 20;
const _fields = {'productId', 'size', 'note', 'quantity'};
const _sizes = {'S', 'M', 'L'};

typedef _Item = ({String size, String note, int quantity});

/// Loopback shop fixtures with an in-memory cart per server instance.
class DemoRoutes {
  /// Reads the editable product fixture from [directory].
  DemoRoutes({required this.directory});

  /// The directory containing `product.dustui`.
  final Directory directory;

  final _cart = <_Item>[];

  /// Handles demo endpoints and returns whether the request was claimed.
  Future<bool> handle(HttpRequest request) async {
    final path = request.uri.path;
    if (!path.startsWith('/ui/')) return false;

    final response = request.response;
    final method = switch (path) {
      '/ui/product' || '/ui/cart' || '/ui/cart/summary' => 'GET',
      '/ui/cart/items' => 'POST',
      _ => null,
    };
    if (method == null) {
      await _reply(response, HttpStatus.notFound, 'Unknown demo route.');
    } else if (request.method != method) {
      response.headers.set(HttpHeaders.allowHeader, method);
      await _reply(response, HttpStatus.methodNotAllowed, 'Use $method.');
    } else if (path == '/ui/product') {
      await _serveProduct(response);
    } else if (path == '/ui/cart/items') {
      await _addItem(request);
    } else {
      await _reply(
        response,
        HttpStatus.ok,
        _cartSource(modal: path == '/ui/cart'),
      );
    }
    return true;
  }

  Future<void> _serveProduct(HttpResponse response) async {
    String source;
    try {
      final file = File.fromUri(directory.uri.resolve('product.dustui'));
      source = await file.readAsString();
    } on FileSystemException {
      await _reply(
        response,
        HttpStatus.internalServerError,
        'Could not read demo/product.dustui.',
      );
      return;
    }
    await _reply(response, HttpStatus.ok, source);
  }

  Future<void> _addItem(HttpRequest request) async {
    final response = request.response;
    if (request.headers.contentType?.mimeType != 'application/json') {
      await _reply(
        response,
        HttpStatus.unsupportedMediaType,
        'Expected application/json.',
      );
      return;
    }

    final bytes = <int>[];
    await for (final chunk in request) {
      if (bytes.length + chunk.length > _maxBodyBytes) {
        await _reply(
          response,
          HttpStatus.requestEntityTooLarge,
          'Cart request is too large.',
        );
        return;
      }
      bytes.addAll(chunk);
    }

    Object? data;
    try {
      data = jsonDecode(utf8.decode(bytes));
    } on FormatException {
      await _reply(response, HttpStatus.badRequest, 'Invalid JSON body.');
      return;
    }
    if (data is! Map<String, Object?> ||
        data.keys.any((key) => !_fields.contains(key)) ||
        data['productId'] != _productId) {
      await _reply(response, HttpStatus.badRequest, 'Invalid cart fields.');
      return;
    }

    final size = data['size'];
    final note = data.containsKey('note') ? data['note'] : '';
    final quantity = data.containsKey('quantity') ? data['quantity'] : 1;
    if (size is! String ||
        !_sizes.contains(size) ||
        note is! String ||
        note.length > 200 ||
        quantity is! int ||
        quantity < 1 ||
        quantity > 10) {
      await _reply(
        response,
        HttpStatus.badRequest,
        'Invalid size, note or quantity.',
      );
      return;
    }
    if (_cart.length >= _maxCartLines) {
      await _reply(
        response,
        HttpStatus.conflict,
        'Demo cart is full. Restart the preview server to clear it.',
      );
      return;
    }

    _cart.add((size: size, note: note, quantity: quantity));
    await _reply(response, HttpStatus.ok, _cartSource(modal: false));
  }

  String _cartSource({required bool modal}) {
    final source = StringBuffer()
      ..writeln(
        modal ? 'col(gap: 8, padding: 16) {' : 'col(id: "cart", gap: 8) {',
      )
      ..writeln(
        '  text(value: "${modal ? 'Your cart' : 'Cart summary'}", size: 18, weight: bold)',
      );
    if (_cart.isEmpty) {
      source.writeln('  text(value: "Your cart is empty.")');
    } else {
      var total = 0;
      for (final item in _cart) {
        final label = '$_productName × ${item.quantity} · Size ${item.size}';
        final subtotal = _price * item.quantity;
        total += subtotal;
        source
          ..writeln('  row(gap: 8) {')
          ..writeln('    text(value: ${jsonEncode(label)})')
          ..writeln('    spacer()')
          ..writeln('    text(value: ${jsonEncode(_money(subtotal))})')
          ..writeln('  }');
        if (item.note.isNotEmpty) {
          source.writeln('  text(value: ${jsonEncode('Note: ${item.note}')})');
        }
      }
      source.writeln(
        '  text(value: ${jsonEncode('Total: ${_money(total)}')}, weight: bold)',
      );
    }
    return (source..writeln('}')).toString();
  }
}

String _money(int amount) => '\$$amount.00';

Future<void> _reply(HttpResponse response, int status, String body) async {
  response.statusCode = status;
  response.headers
    ..contentType = ContentType('text', 'plain', charset: 'utf-8')
    ..set(HttpHeaders.cacheControlHeader, 'no-store');
  response.write(body);
  await response.close();
}
