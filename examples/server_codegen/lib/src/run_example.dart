// coverage:ignore-file

import 'dart:io';

import 'package:dust_server/server.dart';

/// Runs one example until the process receives Ctrl-C.
Future<void> runExample(Router app) async {
  final server = await serve(app, InternetAddress.anyIPv4, 8080);
  stdout.writeln('listening on http://${server.address.host}:${server.port}');

  await ProcessSignal.sigint.watch().first;
  await server.close(drain: const Duration(seconds: 5));
}
