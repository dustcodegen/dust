import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:http/http.dart' as http;

import '../../language.dart';
import '../../renderer.dart';

class PreviewClient {
  final _client = http.Client();

  Future<String> loadSource() => _send('GET', '/ui/product');

  Future<UiNode> request(UiRequest request) async {
    final source = await _send(request.method, request.path, request.body);
    return parseUi(source, fragment: true);
  }

  Future<String> _send(
    String method,
    String path, [
    Map<String, Object?>? body,
  ]) async {
    final uri = Uri.parse(path);
    if (!uri.path.startsWith('/ui/') ||
        uri.hasAuthority ||
        uri.hasScheme ||
        uri.hasQuery ||
        uri.hasFragment ||
        path.contains('\\')) {
      throw const FormatException('Expected a local /ui/ path.');
    }
    final abort = Completer<void>();
    final request = http.AbortableRequest(
      method,
      Uri.base.resolveUri(uri),
      abortTrigger: abort.future,
    )..followRedirects = false;
    request.headers['Accept'] = 'text/plain';
    if (method == 'POST') {
      request.headers['Content-Type'] = 'application/json';
      request.body = jsonEncode(body);
    }
    final deadline = Timer(const Duration(seconds: 10), abort.complete);
    try {
      final response = await _client.send(request);
      if (response.statusCode != 200) {
        throw StateError('Request failed (HTTP ${response.statusCode}).');
      }
      final type = response.headers['content-type']?.split(';').first.trim();
      if (type != 'text/plain') {
        throw const FormatException('Expected a plain text UI document.');
      }
      final bytes = BytesBuilder(copy: false);
      await for (final chunk in response.stream) {
        if (bytes.length + chunk.length > 128 * 1024) {
          throw const FormatException('UI response is larger than 128 KiB.');
        }
        bytes.add(chunk);
      }
      return utf8.decode(bytes.takeBytes());
    } finally {
      deadline.cancel();
      if (!abort.isCompleted) abort.complete();
    }
  }

  void close() => _client.close();
}
