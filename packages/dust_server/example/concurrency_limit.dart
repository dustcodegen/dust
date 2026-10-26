import 'dart:async';
import 'dart:io';

import 'package:dust_server/server.dart';

/// Bounds handler work and sheds load once its queue is full.
///
/// ```bash
/// dart run example/concurrency_limit.dart
/// for i in 1 2 3; do curl -si localhost:8080/work & done; wait
/// ```
// Process wiring is exercised through buildApp in the example suite.
// coverage:ignore-start
Future<void> main() async {
  final server = await serve(buildApp(), InternetAddress.anyIPv4, 8080);
  stdout.writeln('listening on http://${server.address.host}:${server.port}');
}
// coverage:ignore-end

/// Builds one executing slot, one waiting slot, then 503 load shedding.
Router buildApp({Future<void> Function()? work}) {
  return Router()
    ..layer(
      const ConcurrencyLimit(
        maxConcurrentRequests: 1,
        maxQueuedRequests: 1,
      ),
    )
    ..route('/work', get((request) async {
      await (work?.call() ??
          Future<void>.delayed(const Duration(milliseconds: 200)));
      return textResponse('done');
    }));
}
