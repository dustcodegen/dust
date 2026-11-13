import 'dart:io';

import 'package:dust_server/server.dart';
import 'package:server_client_server/websocket.dart';

/// Serves the local authenticated fixture until Ctrl-C.
Future<void> main() async {
  final server =
      await serve(Chat().routes(), InternetAddress.loopbackIPv4, 8080);
  stdout.writeln(
    'Typed chat fixture: http://${server.address.host}:${server.port}',
  );

  await ProcessSignal.sigint.watch().first;
  await server.close(drain: const Duration(seconds: 5));
}
