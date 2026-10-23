import 'dart:convert';
import 'dart:io';

import 'package:dust_server/server.dart';
import 'package:http/http.dart' as http;
import 'package:test/test.dart';

/// A generated example served through a real loopback socket.
final class ExampleApp {
  ExampleApp._(this.server);

  /// Starts [app] on an available local port.
  static Future<ExampleApp> start(Router app) async {
    return ExampleApp._(
      await serve(app, InternetAddress.loopbackIPv4, 0),
    );
  }

  /// The running server.
  final ServerHandle server;

  /// Sends an HTTP request to [path].
  Future<http.Response> send(
    String method,
    String path, {
    Object? body,
    Map<String, String> headers = const {},
  }) async {
    final uri = Uri.parse(
      'http://${server.address.host}:${server.port}$path',
    );
    final requestHeaders = {
      if (body != null && !headers.containsKey('content-type'))
        'content-type': 'application/json',
      ...headers,
    };
    final request = http.Request(method, uri)..headers.addAll(requestHeaders);
    switch (body) {
      case null:
        break;
      case final List<int> bytes:
        request.bodyBytes = bytes;
      case final String text:
        request.body = text;
      default:
        request.body = jsonEncode(body);
    }
    return http.Response.fromStream(await request.send());
  }

  /// Decodes a JSON response object.
  Map<String, Object?> json(http.Response response) {
    return jsonDecode(response.body) as Map<String, Object?>;
  }

  /// Stops the server after in-flight work drains.
  Future<bool> stop() => server.close(drain: const Duration(seconds: 2));
}

/// Starts [app] and registers teardown for the current test.
Future<ExampleApp> serveExample(Router app) async {
  final served = await ExampleApp.start(app);
  addTearDown(served.stop);
  return served;
}
