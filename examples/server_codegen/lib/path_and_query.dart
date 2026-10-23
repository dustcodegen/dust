@Routes('/todos', name: 'todoLookupRoutes')
library;

import 'package:dust_server/server.dart';
import 'package:server_codegen/src/run_example.dart';

part 'path_and_query.g.dart';

/// Runs generated path and query extraction.
///
/// ```bash
/// dart run lib/path_and_query.dart
/// curl 'localhost:8080/todos/7?completed=true'
/// curl -i localhost:8080/todos/not-an-int
/// ```
Future<void> main() => runExample(buildApp()); // coverage:ignore-line

/// Mounts the generated route group.
Router buildApp() => todoLookupRoutes();

/// Reads typed values without touching the raw request.
@GET('/{id}')
Future<Map<String, Object?>> readTodo(
  @Path() int id, {
  @Query('completed') bool? completed,
}) async =>
    {'id': id, 'completed': completed};
