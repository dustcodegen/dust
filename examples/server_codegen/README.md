# Server code generation examples

These examples show the handwritten Dart API that Dust turns into server
routing and extraction code. Generated `.g.dart` files are checked in so each
example can run without installing Rust; run `dust build` after changing an
annotation or handler signature.

```bash
cd examples/server_codegen
dart run lib/hello_world.dart
```

| Use case | Codegen source | Handwritten runtime equivalent |
| :--- | :--- | :--- |
| Plain text and JSON responses | [`lib/hello_world.dart`](lib/hello_world.dart) | [`hello_world.dart`](../../packages/dust_server/example/hello_world.dart) |
| Path and query parameters | [`lib/path_and_query.dart`](lib/path_and_query.dart) | [`path_params.dart`](../../packages/dust_server/example/path_params.dart) and [`query_params.dart`](../../packages/dust_server/example/query_params.dart) |
| JSON body validation | [`lib/json_body.dart`](lib/json_body.dart) | [`json_body.dart`](../../packages/dust_server/example/json_body.dart) |
| Controller grouping and typed state | [`lib/controller_state.dart`](lib/controller_state.dart) | [`state.dart`](../../packages/dust_server/example/state.dart) |
| Bearer auth with a custom extractor | [`lib/bearer_auth.dart`](lib/bearer_auth.dart) | [`bearer_auth.dart`](../../packages/dust_server/example/bearer_auth.dart) |
| Multipart file upload | [`lib/multipart_form.dart`](lib/multipart_form.dart) | [`multipart_form.dart`](../../packages/dust_server/example/multipart_form.dart) |

Each source file includes its own `curl` commands and a `buildApp()` used by
the loopback socket tests:

```bash
dart test
```

Use the [handwritten runtime examples](../../packages/dust_server/example/README.md)
when you do not want code generation. Both styles use the same `Router`,
extractors, responses, and serving runtime.
