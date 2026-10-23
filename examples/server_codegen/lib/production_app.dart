import 'dart:async';
import 'dart:io';

import 'package:dust_server/server.dart';
import 'package:server_codegen/json_body.dart' show noteRoutes;

/// Runs generated routes with the minimum production safety plumbing.
///
/// ```bash
/// dart run lib/production_app.dart
/// curl -i -X POST localhost:8080/notes \
///   -H 'content-type: application/json' -d '{"title":"Ship Dust"}'
/// ```
// Process signals and exit codes are platform wiring covered by server tests.
// coverage:ignore-start
Future<void> main() async {
  final server = await serve(buildApp(), InternetAddress.anyIPv4, 8080);
  stdout.writeln('listening on http://${server.address.host}:${server.port}');

  await Future.any([
    ProcessSignal.sigterm.watch().first,
    ProcessSignal.sigint.watch().first,
  ]);
  final drained = await server.close(drain: const Duration(seconds: 15));
  if (!drained) {
    stderr.writeln('shutdown deadline passed with work still running');
    exitCode = 1;
  }
}
// coverage:ignore-end

/// Wraps generated routes with limits, request ids, logs, and safe headers.
Router buildApp({
  void Function(AccessRecord)? onRecord,
  void Function(Object error, StackTrace stack)? onError,
}) {
  return Router(bodyLimit: 64 * 1024, onError: onError)
    ..layer(const RequestId())
    ..layer(AccessLog(onRecord ?? _printAccess))
    ..layer(const SecurityHeaders())
    ..layer(const RequestTimeout(Duration(seconds: 10)))
    ..merge(noteRoutes());
}

/// Writes one bounded access record without query-string secrets.
// coverage:ignore-start
void _printAccess(AccessRecord record) => stdout.writeln(record);
// coverage:ignore-end
