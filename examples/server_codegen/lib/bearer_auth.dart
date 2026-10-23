@Routes('/', name: 'authRoutes')
library;

import 'package:dust_server/server.dart';
import 'package:server_codegen/src/run_example.dart';

part 'bearer_auth.g.dart';

/// Runs a generated route with a custom bearer extractor.
///
/// ```bash
/// dart run lib/bearer_auth.dart
/// curl localhost:8080/me -H 'authorization: Bearer demo-token'
/// curl -i localhost:8080/me
/// ```
Future<void> main() => runExample(buildApp()); // coverage:ignore-line

/// Mounts the generated route group.
Router buildApp() => authRoutes();

/// Returns the identity produced by [DemoAuth].
@GET('/me')
Future<Map<String, Object?>> me(@Extract(DemoAuth) String user) async => {
      'user': user,
    };

/// Turns the example bearer token into a user name.
final class DemoAuth implements FromRequestParts<String> {
  /// Creates the stateless extractor used by generated code.
  const DemoAuth();

  @override
  Future<Result<String, Rejection>> extract(Request request) async {
    final token = await const BearerTokenExtractable().extract(request);
    return switch (token) {
      Err(:final error) => Err(error),
      Ok(value: 'demo-token') => const Ok('Ada'),
      Ok() => const Err(Rejection.forbidden('unknown token')),
    };
  }
}
