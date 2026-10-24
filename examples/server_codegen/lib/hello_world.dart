@Routes('/', name: 'helloWorldRoutes')
library;

import 'package:dust_server/server.dart';
import 'package:server_codegen/src/run_example.dart';

part 'hello_world.g.dart';

/// Runs the smallest generated server.
///
/// ```bash
/// dart run lib/hello_world.dart
/// curl localhost:8080/
/// curl localhost:8080/json
/// ```
Future<void> main() => runExample(buildApp()); // coverage:ignore-line

/// Mounts the generated route group.
Router buildApp() => helloWorldRoutes();

/// Returns plain text from a generated handler.
@GET('/')
Future<String> hello() async => 'Hello, world!';

/// Returns JSON from a generated handler.
@GET('/json')
Future<Map<String, Object?>> greeting() async => {
      'greeting': 'Hello, world!',
    };
