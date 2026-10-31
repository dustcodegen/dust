import 'dart:io';

import 'package:dust_server/server.dart';
import 'package:server_client_server/client_edges.dart';

Future<void> main() async {
  final server = await serve(
    Transport().routes(),
    InternetAddress.anyIPv4,
    8080,
  );
  stdout.writeln('ready ${server.address.host}:${server.port}');

  await ProcessSignal.sigterm.watch().first;
  await server.close(drain: const Duration(seconds: 5));
}
