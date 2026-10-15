import 'dart:io';

import 'package:path_provider/path_provider.dart';

/// Rejects missing phases and path syntax before touching test storage.
void validateProbeConfig(String run, String phase) {
  if (!RegExp(r'^[a-zA-Z0-9_-]{1,64}$').hasMatch(run) ||
      !const ['write', 'verify'].contains(phase)) {
    throw ArgumentError(
        'Provide a unique probe run ID and write/verify phase.');
  }
}

/// Uses durable application support storage, never the platform code cache.
Future<Directory> probeDirectory(String run, String phase) async {
  validateProbeConfig(run, phase);
  final support = await getApplicationSupportDirectory();
  return Directory('${support.path}/dust-sqlite-probe-$run');
}
