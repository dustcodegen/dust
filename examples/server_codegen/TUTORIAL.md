# Dust server codegen tutorial

This tutorial starts with one generated route and grows it only where an HTTP
application needs another boundary. Generated `.g.dart` files are checked in,
so application users need Rust only when annotations or handler signatures
change.

## 1. Create a route module

```dart
@Routes('/todos', name: 'todoRoutes')
library;

import 'package:dust_server/server.dart';

part 'todos.g.dart';

@GET('/{id}')
Future<Map<String, Object?>> read(@Path() int id) async => {'id': id};
```

Run `dust build`, then mount the generated group:

```dart
final app = Router()..merge(todoRoutes());
await serve(app, InternetAddress.anyIPv4, 8080);
```

Use `@Routes(...)` for top-level functions. Use `@Controller(...)` when handlers share
constructor dependencies. Both generate the same router primitives.

## 2. Declare request inputs

Handler parameters are the request contract:

```dart
@GET('/{id}')
Future<Map<String, Object?>> read(
  @Path() int id, {
  @Query('details') bool? details,
  @Header('x-request-id') String? requestId,
}) async => {
  'id': id,
  'details': details,
  'requestId': requestId,
};
```

Group related query fields without writing a decoder:

```dart
@Derive([Deserialize(), Validate()])
final class TodoFilter with _$TodoFilter {
  const TodoFilter({required this.owner, this.page = 1, this.tags = const []});

  final String owner;

  @SerDe(defaultValue: 1)
  @Validate(range: Range(min: 1))
  final int page;

  @SerDe(defaultValue: <String>[], aliases: ['labels'])
  final List<String> tags;
}

@GET('/search')
Future<List<Todo>> search(@Query() TodoFilter filter) async => repository.search(filter);
```

- A non-nullable scalar is required and rejects invalid input with 400.
- A nullable scalar is optional.
- `List<T>` on `@Query(...)` reads repeated values.
- `@Query() TodoFilter filter` decodes a flat `Deserialize()` model, including
  SerDe names, aliases, defaults, enums, and repeated lists. A derived
  `Validate()` contract runs after decoding.
- `@Queries()` remains source-compatible for existing dynamic-map handlers;
  new typed handlers should use `@Query()`.
- `@HeaderMap()`, `@RawQuery()`, `@Peer()`, `Request`, and
  `RequestParts` expose broader request data when it is genuinely needed.

See [request_data.dart](lib/request_data.dart) for a runnable request-data
example and [path_and_query.dart](lib/path_and_query.dart) for both named and
model query extraction.

## 3. Decode and validate JSON once

```dart
@Derive([Deserialize(), Validate()])
final class CreateTodo with _$CreateTodo {
  const CreateTodo({required this.title});

  @Validate(length: Length(min: 1))
  final String title;
}

@POST('/', status: 201)
Future<Map<String, Object?>> create(@Body() CreateTodo input) async =>
    {'title': input.title};
```

`Deserialize()` generates the JSON decoder used by server codegen. The field
declaration supplies the validation type, so do not repeat it as
`@Validate<String>`. A same-library `Validate()` model is validated after
decoding; malformed JSON and invalid models remain distinct 400/422 failures.

See [json_body.dart](lib/json_body.dart).

## 4. Select one body mode

A request body is read once. Pick the representation the endpoint owns:

| Body contract | Generated declaration |
| :--- | :--- |
| JSON object or model | `@Body()` |
| URL-encoded fields | `@FormUrlEncoded()` plus `@Field(...)` |
| Buffered multipart fields/files | `@MultiPart()` plus `@Part(...)` |
| Text, bytes, or byte stream | `@RawBody()` with the matching Dart type |

See [forms_and_raw.dart](lib/forms_and_raw.dart) and
[multipart_form.dart](lib/multipart_form.dart).

### Why `@FormUrlEncoded()` stays separate

`@POST(...)` declares transport: method, path, status, metadata, and middleware.
`@FormUrlEncoded()` selects how the one request body is decoded. Keeping those
jobs separate matches Retrofit and lets the same body mode work with `POST`,
`PUT`, or `PATCH` without adding flags to every verb.

Do not collapse it into `@POST('/form', formUrlEncoded: true)`. Multipart would
need another Boolean, making the impossible combination `formUrlEncoded: true,
multipart: true` representable. The two annotations make that conflict visible
to Dust and produce one clear build diagnostic.

## 5. Choose extraction failure behavior

```dart
Future<void> handler(
  @Extract(BearerAuth) AuthUser required,
  @Extract(BearerAuth) Option<AuthUser> optional,
  @Extract(BearerAuth) Result<AuthUser, Rejection> inspected,
) async {}
```

- Plain `T` short-circuits with the extractor rejection.
- `Option<T>` turns client rejections into `None`; server faults still fail.
- `Result<T, Rejection>` hands either outcome to the handler.

The wrappers work with built-in extractors too. See
[bearer_auth.dart](lib/bearer_auth.dart).

## 6. Return the value you mean

Generated handlers dispatch common response shapes directly:

| Handler return | HTTP behavior |
| :--- | :--- |
| `String` | text response |
| JSON-compatible value | JSON response |
| `void` or `Unit` | 204 by default |
| `Option<T>` | value or 404 |
| `Result<T, E>` | success or error response |
| `Response` | preserved unchanged |
| `Uint8List` / `Stream<List<int>>` | binary or streamed body |

See [verbs_and_responses.dart](lib/verbs_and_responses.dart).

## 7. Compose generated routes for production

Generation stops at the router boundary. The application still owns limits,
operational layers, serving, and shutdown:

```dart
import 'dart:io';

final app = Router(
  bodyLimit: 64 * 1024,
  onError: (error, stack) => stderr.writeln('$error\n$stack'),
)
  ..layer(const RequestId())
  ..layer(AccessLog((record) => stdout.writeln(record)))
  ..layer(const SecurityHeaders())
  ..layer(const RequestTimeout(Duration(seconds: 10)))
  ..merge(noteRoutes());
```

The generated extraction contract stays stable at the edge:

| Failure | Status |
| :--- | :--- |
| Wrong `content-type` | 415 |
| Malformed JSON/form/multipart syntax | 400 |
| Decoded value has the wrong shape | 422 |
| `Validate()` constraint fails | 422 |
| Body exceeds the configured limit | 413 |
| Handwritten handler throws unexpectedly | 500, detail reported only through `onError` |

See [production_app.dart](lib/production_app.dart) for the runnable composition.

## 8. Generate, check, and test

```bash
dust build
dust check
dart analyze --fatal-infos
dart test
```

`dust check` proves checked-in output matches the source without rewriting it.
The tests in this package serve every generated router on a real loopback
socket.

## What stays handwritten

Codegen owns route declarations, extraction, validation dispatch, and response
dispatch. Application composition and operational features remain explicit:

- server address, port, TLS, isolates, and graceful shutdown;
- global middleware, CORS policy, limits, tracing, and observability;
- WebSockets, SSE, static files, templates, and background tasks;
- database connections, transactions, and migrations.

Those are runtime concerns rather than handler boilerplate. Use the
[handwritten runtime examples](../../packages/dust_server/example/README.md)
for them.
