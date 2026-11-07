import 'dart:io';

import 'demo_routes.dart';

/// Serves the lab and mock API from one loopback origin.
Future<HttpServer> startPreviewServer({
  required Directory bundle,
  required Directory demo,
  required int port,
  File? source,
}) async {
  final root = await bundle.resolveSymbolicLinks();
  final server = await HttpServer.bind(InternetAddress.loopbackIPv4, port);
  final origin = 'http://127.0.0.1:${server.port}';
  final demoRoutes = DemoRoutes(directory: demo);
  server.listen((request) async {
    try {
      request.response.headers
        ..set(HttpHeaders.cacheControlHeader, 'no-store')
        ..set('X-Content-Type-Options', 'nosniff');
      final host = request.headers.value(HttpHeaders.hostHeader);
      final requestOrigin = request.headers.value('origin');
      if (host != '127.0.0.1:${server.port}' ||
          (requestOrigin != null && requestOrigin != origin)) {
        await _status(request, HttpStatus.forbidden);
        return;
      }
      if (source != null &&
          request.uri.path == '/ui/product' &&
          request.method == 'GET') {
        // Invalid syntax is passed through so the editor can diagnose it.
        if (await source.length() > 128 * 1024) {
          await _status(request, HttpStatus.requestEntityTooLarge);
          return;
        }
        request.response.headers.contentType = ContentType.text;
        await request.response.addStream(source.openRead());
        await request.response.close();
        return;
      }
      if (await demoRoutes.handle(request)) return;
      await _serveFile(request, root);
    } on FileSystemException {
      await _status(request, HttpStatus.notFound);
    } on Object {
      await _status(request, HttpStatus.internalServerError);
    }
  });
  return server;
}

Future<void> _serveFile(HttpRequest request, String root) async {
  if (request.method != 'GET' && request.method != 'HEAD') {
    request.response.headers.set('Allow', 'GET, HEAD');
    await _status(request, HttpStatus.methodNotAllowed);
    return;
  }
  final segments = request.uri.path == '/'
      ? const ['index.html']
      : request.uri.pathSegments;
  if (segments.any(
    (part) =>
        part.isEmpty ||
        part.startsWith('.') ||
        part.contains('/') ||
        part.contains('\\') ||
        part.contains('\x00'),
  )) {
    await _status(request, HttpStatus.notFound);
    return;
  }
  final path = [root, ...segments].join(Platform.pathSeparator);
  final resolved = await File(path).resolveSymbolicLinks();
  if (!resolved.startsWith('$root${Platform.pathSeparator}')) {
    await _status(request, HttpStatus.notFound);
    return;
  }
  final file = File(resolved);
  request.response.headers.contentType =
      _types[resolved.split('.').last] ?? ContentType.binary;
  request.response.contentLength = await file.length();
  if (request.method == 'GET') {
    await request.response.addStream(file.openRead());
  }
  await request.response.close();
}

Future<void> _status(HttpRequest request, int status) async {
  request.response.statusCode = status;
  await request.response.close();
}

final _types = {
  'html': ContentType.html,
  'css': ContentType('text', 'css', charset: 'utf-8'),
  'js': ContentType('text', 'javascript', charset: 'utf-8'),
  'json': ContentType.json,
  'wasm': ContentType('application', 'wasm'),
  'woff2': ContentType('font', 'woff2'),
  'ttf': ContentType('font', 'ttf'),
  'otf': ContentType('font', 'otf'),
  'png': ContentType('image', 'png'),
};
