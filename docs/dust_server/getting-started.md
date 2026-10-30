# Getting started

## Install

```bash
dart pub add dust_server
```

Require Dust 0.3.0 or newer:

```yaml
dependencies:
  dust_server: ^0.3.0
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

### Generate a client package from the routes

Put the server and client in the same Dart pub workspace. The client package
must declare `dio`, then select its exact `pubspec.yaml` name in the server's
`dust.yaml`:

```yaml
server:
  client_package: todo_client
```

To split clients, map destination packages to existing route tags instead:

```yaml
server:
  client_packages:
    public_client: [public]
    admin_client: [admin]
```

Tags declared by `@Routes` and `@Controller` are inherited; tags on `@GET`,
`@POST`, and the other verb annotations apply to one route. A route is emitted
to every destination matching one of its final tags. Untagged or unmatched
routes remain server-only. The singular and plural settings are mutually
exclusive.

`dust build` writes a generated barrel per route library under the client
package. For example, `server/lib/items.dart` generates
`todo_client/lib/src/generated/items_client.g.dart`. Every controller gets a
separate client class in its own generated file; top-level `@Routes`
functions get a separate module client. The barrel exports all of them.
`@Routes` and `@Controller` prefixes are included in each request path. No
`@HttpContract`, duplicate interface, or aggregate client class is required.

Export the generated route barrels you want from the client package entrypoint:

```dart
// todo_client/lib/todo_client.dart
export 'src/generated/items_client.g.dart';
```

Then construct the generated route or controller client with your configured
`Dio` instance:

```dart
final items = ItemsRoutesClient(
  Dio(BaseOptions(baseUrl: 'https://api.example.com')),
);
final item = await items.read(42, details: true);
```

Path, query, header, JSON body, form and multipart inputs become client method
parameters. `@State`, `@Peer`, `Request`, `RequestParts`, raw request inputs and
custom extractors remain server-only and are omitted. Dust rejects private or
unreachable client types before replacing generated output. `dust check`
reports a changed or missing client as stale.

For a typed whole-query model, declare the model in a package both sides can
import. The generated client package itself can own that contract. Derive both
directions because the client serializes it and the server deserializes it:

```dart
@Derive([Serialize(), Deserialize()])
final class TodoFilter with _$TodoFilter {
  const TodoFilter({required this.owner, this.page = 1});

  final String owner;

  @SerDe(defaultValue: 1)
  final int page;
}
```

Import it with a prefix in the server route and use the normal `@Query()` API:

```dart
import 'package:todo_client/contracts.dart' as contracts;

@GET('/search')
Future<List<Todo>> search(@Query() contracts.TodoFilter filter) async =>
    repository.search(filter);
```

The generated client accepts the same `TodoFilter`. SerDe rename rules define
the encoded keys, aliases remain decode-only, nullable values are omitted, and
list values remain repeated query parameters.

JSON bodies and responses use the same shared-model rule. Put the contracts in
a package imported by both sides, derive both directions, and use the types
directly in the server route:

```dart
@Derive([Serialize(), Deserialize(), Validate()])
final class CreateTodo with _$CreateTodo {
  const CreateTodo({required this.title});

  @Validate(length: Length(min: 1))
  final String title;
}

@Derive([Serialize(), Deserialize()])
final class Todo with _$Todo {
  const Todo({required this.id, required this.title});

  final String id;
  final String title;
}
```

```dart
import 'package:todo_client/contracts.dart' as contracts;

@POST('/', status: 201)
Future<contracts.Todo> create(
  @Body() contracts.CreateTodo input,
) async => repository.create(input);
```

The server uses the generated public deserializer and applies `Validate()`.
The generated client calls `toJson()` for the request and returns a typed
`Todo`; `List<Todo>` and `Todo?` responses are decoded the same way. A known
shared model missing a required derive fails during generation. Existing
handwritten `toJson`, `fromJson`, and static `deserialize` contracts remain
supported.

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
`@Query()` decodes the complete query string into a local or prefixed imported
model deriving `Deserialize()`, using its SerDe names, aliases, defaults, enums,
and flat list fields. A model deriving `Validate()` is validated after decoding.
Generated clients additionally require `Serialize()`. Existing `@Queries()` map
handlers remain compatible, but new typed handlers use `@Query()`.

A same-library JSON `@Body()` model deriving `Deserialize()` uses its generated
private decoder. A prefixed imported JSON model deriving `Deserialize()` uses
its public generated deserializer; deriving `Validate()` adds automatic
post-decode validation. Compatibility models may expose static `deserialize`.
Form values are strings rather than JSON values.

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

## Generated SQLite Background Opening

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
