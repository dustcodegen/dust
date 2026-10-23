import 'package:dust_server/server.dart';
import 'package:server_codegen/src/run_example.dart';

part 'controller_state.g.dart';

/// Runs a generated controller with typed application state.
///
/// ```bash
/// dart run lib/controller_state.dart
/// curl localhost:8080/greetings/Ada
/// ```
Future<void> main() => runExample(buildApp()); // coverage:ignore-line

/// Mounts the controller and attaches its shared state by type.
Router buildApp() => Router()
  ..merge(GreetingController().routes())
  ..withState(const GreetingConfig('Hello'));

/// Groups greeting endpoints under one path.
@Controller('/greetings')
final class GreetingController with _$GreetingController {
  /// Reads [GreetingConfig] from the router and a name from the path.
  @GET('/{name}')
  Future<Map<String, Object?>> greet(
    @Path() String name,
    @State() GreetingConfig config,
  ) async =>
      {'message': '${config.prefix}, $name!'};
}

/// Configuration created once and shared with handlers.
final class GreetingConfig {
  /// Creates greeting configuration.
  const GreetingConfig(this.prefix);

  /// Text written before the requested name.
  final String prefix;
}
