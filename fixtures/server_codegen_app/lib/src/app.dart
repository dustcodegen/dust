import 'dart:io';

import 'package:dust_server/server.dart';
import 'package:server_codegen_app/src/basic_routes.dart';
import 'package:server_codegen_app/src/todo_controller.dart';

/// Builds the production-shaped fixture around generated route groups.
Router buildApp({
  int bodyLimit = 64,
  void Function(Object error, StackTrace stack)? onError,
}) {
  final generated = Router()
    ..merge(basicRoutes())
    ..merge(TodoController().routes())
    ..merge(todoControllerRoutes())
    ..withState(const TodoState('fixture'));

  return Router(bodyLimit: bodyLimit, onError: onError)
    ..layer(const TraceLayer('root'))
    ..nest('/api/v1/', generated);
}

/// Starts the same router used by the command-line entrypoint.
Future<ServerHandle> startServer({
  InternetAddress? address,
  int port = 8080,
  int bodyLimit = 64,
  void Function(Object error, StackTrace stack)? onError,
}) {
  return serve(
    buildApp(bodyLimit: bodyLimit, onError: onError),
    address ?? InternetAddress.anyIPv4,
    port,
  );
}
