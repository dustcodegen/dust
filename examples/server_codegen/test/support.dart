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
  }) {
    final uri = Uri.parse(
      'http://${server.address.host}:${server.port}$path',
    );
    final encoded = switch (body) {
      null => null,
      final String text => text,
      _ => jsonEncode(body),
    };
    final requestHeaders = {
      if (body != null && !headers.containsKey('content-type'))
        'content-type': 'application/json',
      ...headers,
    };
    return switch (method) {
      'GET' => http.get(uri, headers: requestHeaders),
      'POST' => http.post(
          uri,
          headers: requestHeaders,
          body: encoded,
        ),
      _ => throw ArgumentError.value(method, 'method'),
    };
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
