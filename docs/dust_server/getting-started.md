# Getting started

## Install

```bash
dart pub add dust_server
```

The current stable line is 0.2.x. To state the minimum explicitly:

```yaml
dependencies:
  dust_server: ^0.2.1
```

Inside this workspace, depend on it by path instead:

```yaml
dependencies:
  dust_server:
    path: ../../packages/dust_server
```

## Entry points

Import everything, or only the part a file needs. Every symbol is reachable
from `server.dart`; the narrower libraries exist so a composition file does not
pull in annotations, and a handler file does not pull in the server.

| Library | Contents |
| :--- | :--- |
| `package:dust_server/server.dart` | everything below |
| `package:dust_server/annotations.dart` | `@Controller(...)`, the verbs, parameter annotations, `@Routes(...)` |
| `package:dust_server/extraction.dart` | `FromRequestParts`, `FromRequest`, the built-in extractors, coercion |
| `package:dust_server/response.dart` | `Rejection`, `IntoResponse`, encoders, `guard`, `ServerErrors` |
| `package:dust_server/router.dart` | `Router`, `Route`, `MethodRouter`, the verb builders, `Layer` |
| `package:dust_server/layers.dart` | `RequestTimeout`, `RequestId`, `AccessLog` |
| `package:dust_server/serving.dart` | `serve`, `serveIsolates`, static files |
| `package:dust_server/ws.dart` | WebSocket routes and sessions |
| `package:dust_server/templating.dart` | template engines and HTML responses |

`test/libraries_test.dart` holds each one to that contract: a file importing
only `router.dart` has to compile without `server.dart`.

## Layout

`lib/src` is split by feature, and no file passes 180 lines. `test` mirrors it.

| Folder | Contents |
| :--- | :--- |
| `annotations/` | what the generator will read; nothing here has behavior |
| `request/` | `RequestParts`, and the coercion every keyed extractor uses |
| `extraction/` | the extractor interfaces and one file per built-in |
| `response/` | rejections, encoders, HTML, error reporting |
| `router/` | the router, matcher, composer, flattener, path helpers |
| `layers/` | the middleware that ships |
| `serving/` | running, draining, clustering, static files |
| `template/` | the template seam and its adapter |
| `ws/` | WebSocket routes and sessions |

Each folder carries a barrel, so a public entry point is one export line and the
folder decides what stays internal.

## Trying it

Generate a small function-based route module:

```dart
@Routes('/items', name: 'itemRoutes')
library;

import 'package:dust_server/server.dart';

part 'items.g.dart';

@GET('/{id}')
Future<Map<String, Object?>> read(
  @Path() int id, {
  @Query('details') bool? details,
}) async => {'id': id, 'details': details ?? false};

@POST('/', status: 201)
Future<Map<String, Object?>> create(@Query('name') String name) async =>
    {'name': name};

@DELETE('/{id}')
Future<void> delete(@Path() int id) async {}
```

```bash
dust build
```

Mount `itemRoutes()` with `Router.merge`. Use `@Controller(...)` when class-owned
dependencies make a class clearer. Generated handlers also support typed
`@Query()` models, compatibility `@Queries()` maps, `@Header(...)`,
`@HeaderMap()`, `@Body()`, `@Field(...)`,
`@Part(...)`, `@RawBody()`, `@RawQuery()`, `@State()`, `@Peer()`, bare
`Request`/`RequestParts`, and `@Extract(Type)`. Declare an extractor parameter
as `Option<T>` to collapse client rejections to `None`, or as
`Result<T, Rejection>` to pass the outcome to the handler; the same wrappers
work with built-in extractors. Runnable focused examples and request tests live
in [`examples/server_codegen`](../../examples/server_codegen/README.md). The
exhaustive internal regression fixture remains in
[`fixtures/server_codegen_app`](../../fixtures/server_codegen_app).

Automatic `Validate()` wrapping currently requires the body model and handler
to be declared in the same Dart library. An imported body model still decodes,
but validate it explicitly in the handler until cross-library derive facts are
available.

## Generated annotation reference

These are the 24 annotations currently consumed by server codegen. The forms
below match their Dart declarations exactly.

| Target | Supported annotations |
| :--- | :--- |
| Library | `@Routes(path, name:, tags:, middleware:)` |
| Class | `@Controller(path, tags:, middleware:)` |
| Method or function | `@GET(path, ...)`, `@POST(path, ...)`, `@PUT(path, ...)`, `@PATCH(path, ...)`, `@DELETE(path, ...)`, `@HEAD(path, ...)`, `@OPTIONS(path, ...)` |
| Body mode | `@FormUrlEncoded()`, `@MultiPart()` |
| Parameter | `@Extract(Type)`, `@Path([name])`, `@Query([name])`, `@Queries()`, `@RawQuery()`, `@Header(name)`, `@HeaderMap()`, `@Body()`, `@Field(name)`, `@Part(name)`, `@RawBody()`, `@State()`, `@Peer()` |

Every verb accepts `status:`, `summary:`, `description:`, `tags:`,
`middleware:`, `operationId:`, `deprecated:`, and `hidden:`. A handler has one
verb and at most one body mode. `@FormUrlEncoded()` and `@MultiPart()` stay
separate from the verb because the encoding is independent of the HTTP method.

`@Query(name)` reads and coerces one scalar or repeated list. An unnamed
`@Query()` decodes the complete query string into a same-library model deriving
`Deserialize()`, using its SerDe names, aliases, defaults, enums, and flat list
fields. A model deriving `Validate()` is validated after decoding. Existing
`@Queries()` map handlers remain compatible, but new typed handlers use
`@Query()`.

A same-library JSON `@Body()` model deriving `Deserialize()` uses its generated
private decoder. Imported JSON and form models expose a compatible static
`deserialize` function; form values are strings rather than JSON values.

The existing handwritten examples remain the non-codegen track:

```bash
cd packages/dust_server
dart run example/hello_world.dart
```

```bash
curl -s localhost:8080/           # Hello, world!
curl -s localhost:8080/hello/ada  # Hello, ada!
curl -s localhost:8080/json       # {"greeting":"Hello, world!"}
```

A handler returns the value it produced and the verb builder encodes it: a
`String` goes out as `text/plain`, and anything else as JSON. That is axum's
rule. To send a JSON string specifically, wrap it — `['Hello']` or
`{'greeting': 'Hello'}` — or call `jsonResponse`.

Every example runs the same way. The
[index](../../packages/dust_server/example/README.md) lists all 51 by the
question each one answers.

## Generated SQLite Background Opening (Unreleased)

The native server fixture also opts its database declaration into
`await AppDatabase.openBackground(path, options: appOptions)`. Opening,
embedded migration verification/application and SQL run on one persistent
worker; DAOs and static non-DAO SQLx query helpers keep the same executor API.
Normal `AppDatabase.open` remains synchronous.

```bash
cd fixtures/server_app
dart run example/background_database.dart
dart test test/background_database_test.dart
```

The [example](../../fixtures/server_app/example/background_database.dart) uses
the generated facade, a transaction, a checked DAO, a checked non-DAO total and
async unsafe SQL. Its output is asserted exactly, and the regression serves the
real application's HTTP checkout flow over loopback with the worker connection.
This is not a live-service load test or device/crash evidence. Worker ownership
still fails fast for unrelated work during a transaction; this feature adds no
queue, retry policy or pool. See the
[opt-in and compatibility guide](../usage/db.md#generated-background-opening).
Until v0.3.0 is published, use matching current-source CLI/runtime packages.
