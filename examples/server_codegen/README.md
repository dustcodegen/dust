# Server code generation examples

These examples show the handwritten Dart API that Dust turns into server
routing and extraction code. Generated `.g.dart` files are checked in so each
example can run without installing Rust; run `dust build` after changing an
annotation or handler signature.

```bash
cd examples/server_codegen
dart run lib/hello_world.dart
```

Start with the [step-by-step tutorial](TUTORIAL.md), then use this table as the
runnable reference.

| Use case | Codegen source | Handwritten runtime equivalent |
| :--- | :--- | :--- |
| Plain text and JSON responses | [`lib/hello_world.dart`](lib/hello_world.dart) | [`hello_world.dart`](../../packages/dust_server/example/hello_world.dart) |
| Path and query parameters | [`lib/path_and_query.dart`](lib/path_and_query.dart) | [`path_params.dart`](../../packages/dust_server/example/path_params.dart) and [`query_params.dart`](../../packages/dust_server/example/query_params.dart) |
| JSON body validation | [`lib/json_body.dart`](lib/json_body.dart) | [`json_body.dart`](../../packages/dust_server/example/json_body.dart) |
| Controller grouping and typed state | [`lib/controller_state.dart`](lib/controller_state.dart) | [`state.dart`](../../packages/dust_server/example/state.dart) |
| Bearer auth with a custom extractor | [`lib/bearer_auth.dart`](lib/bearer_auth.dart) | [`bearer_auth.dart`](../../packages/dust_server/example/bearer_auth.dart) |
| Multipart file upload | [`lib/multipart_form.dart`](lib/multipart_form.dart) | [`multipart_form.dart`](../../packages/dust_server/example/multipart_form.dart) |
| Query/header maps, peer and request parts | [`lib/request_data.dart`](lib/request_data.dart) | [`query_params.dart`](../../packages/dust_server/example/query_params.dart), [`headers_and_host.dart`](../../packages/dust_server/example/headers_and_host.dart), and [`client_ip.dart`](../../packages/dust_server/example/client_ip.dart) |
| URL-encoded forms and raw bodies | [`lib/forms_and_raw.dart`](lib/forms_and_raw.dart) | [`form_body.dart`](../../packages/dust_server/example/form_body.dart) and [`parse_body_by_content_type.dart`](../../packages/dust_server/example/parse_body_by_content_type.dart) |
| PUT/PATCH/DELETE/HEAD/OPTIONS and response wrappers | [`lib/verbs_and_responses.dart`](lib/verbs_and_responses.dart) | [`routing.dart`](../../packages/dust_server/example/routing.dart), [`handle_head_request.dart`](../../packages/dust_server/example/handle_head_request.dart), and [`error_handling.dart`](../../packages/dust_server/example/error_handling.dart) |

Each source file includes its own `curl` commands and a `buildApp()` used by
the loopback socket tests:

```bash
dart test
```

Use the [handwritten runtime examples](../../packages/dust_server/example/README.md)
when you do not want code generation. Both styles use the same `Router`,
extractors, responses, and serving runtime.

Codegen now has runnable coverage for route modules, controllers, every HTTP
verb, route metadata and middleware, built-in parameter/body extraction,
required/optional/fallible extraction, validation, and standard response
dispatch. Operational runtime features remain handwritten by design; the
[tutorial](TUTORIAL.md#what-stays-handwritten) names that boundary.
