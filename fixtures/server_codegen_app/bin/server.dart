import 'dart:io';

import 'package:server_codegen_app/server_codegen_app.dart';

Future<void> main(List<String> arguments) async {
  final port = arguments.isEmpty ? 8080 : int.parse(arguments.single);
  final server = await startServer(port: port);
  stdout.writeln('listening on http://${server.address.host}:${server.port}');
  await ProcessSignal.sigterm.watch().first;
  await server.close(drain: const Duration(seconds: 15));
}
