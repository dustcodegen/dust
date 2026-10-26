import 'dart:io';

import 'package:dust_server/server.dart';

/// Limits each API key without trusting a spoofable proxy header.
///
/// ```bash
/// dart run example/rate_limit.dart
/// curl -i localhost:8080/work -H 'x-api-key: demo' # 200
/// curl -i localhost:8080/work -H 'x-api-key: demo' # 429
/// ```
// Process wiring is exercised through buildApp in the example suite.
// coverage:ignore-start
Future<void> main() async {
  final server = await serve(buildApp(), InternetAddress.anyIPv4, 8080);
  stdout.writeln('listening on http://${server.address.host}:${server.port}');
}
// coverage:ignore-end

/// Uses a trusted application credential as the bucket identity.
String clientKey(Request request) =>
    request.headers['x-api-key'] ?? 'anonymous';

/// Builds a one-request-per-minute policy for each identity.
Router buildApp() {
  return Router()
    ..layer(
      const RateLimit(
        maxRequests: 1,
        window: Duration(minutes: 1),
        keyBy: clientKey,
      ),
    )
    ..route('/work', get((request) async => {'ok': true}));
}
