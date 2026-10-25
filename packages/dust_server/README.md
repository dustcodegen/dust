# dust_server

[![pub package](https://img.shields.io/pub/v/dust_server.svg)](https://pub.dev/packages/dust_server)
[![pub points](https://img.shields.io/pub/points/dust_server)](https://pub.dev/packages/dust_server/score)
[![downloads](https://img.shields.io/pub/dm/dust_server)](https://pub.dev/packages/dust_server)

Runtime support for Dust-generated Dart HTTP servers on `shelf`.

> [!IMPORTANT]
> **0.2.1 stable runtime.** The current source checkout also generates wiring
> for controllers and top-level route modules: typed extraction, validation,
> response dispatch, middleware, guarded failures, and `RouteMetadata` visible
> through `Router.describe()`. The generator is not in the published 0.2.1
> binaries yet. OpenAPI generation remains a separate package.

## Documentation

Full documentation lives in [docs/dust_server](../../docs/dust_server/README.md).

| Section | Covers |
| :--- | :--- |
| [Getting started](../../docs/dust_server/getting-started.md) | the smallest server, entry points, layout |
| [Routing](../../docs/dust_server/routing.md) | paths, methods, nesting, mounting, matching rules, speed |
| [Extraction](../../docs/dust_server/extraction.md) | the built-in extractors, coercion, writing one |
| [Responses](../../docs/dust_server/responses.md) | encoders, rejections, `guard`, error reporting |
| [WebSockets](../../docs/dust_server/websockets.md) | upgrades, sessions, close codes |
| [Rendering](../../docs/dust_server/rendering.md) | templates, static files, single-page apps |
| [Serving](../../docs/dust_server/serving.md) | draining, TLS, isolate clustering |
| [Middleware](../../docs/dust_server/middleware.md) | layers, ordering, what ships |
| [Testing](../../docs/dust_server/testing.md) | running the suite, testing an application |
| [Dependencies](../../docs/dust_server/dependencies.md) | what is reused, and what is not |

## At a glance

Start with top-level functions when routes do not need class-owned dependencies:

```dart
@Routes('/items', name: 'itemRoutes')
library items;

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

Use `@Query() Filter filter` for a flat same-library model deriving
`Deserialize()`. Generated code applies SerDe names, aliases, defaults, enums,
and repeated list values; deriving `Validate()` adds post-decode validation.
Named `@Query('details')` parameters remain the smallest choice for one value.

Run `dust build`, then mount `itemRoutes()` in the application router. Use
`@Controller(...)` only when a class is the clearer home for shared dependencies.
Start with the focused
[`examples/server_codegen`](../../examples/server_codegen/README.md); the
complete internal regression fixture is
[`fixtures/server_codegen_app`](../../fixtures/server_codegen_app).

Metadata declared on a verb is available after normal mounting:

```dart
@GET('/{id}', summary: 'Read a todo', tags: ['todos'])
Future<Todo> read(@Path() int id) async => repository.read(id);

final route = app.describe().first;
final metadata = route.metadataOf<RouteMetadata>();
```

Runtime composition remains available directly:

```dart
final app = Router()
  ..layer(const RequestId())
  ..route('/todos/{id}', get(readTodo))
  ..route('/chat/{room}', ws(joinRoom))
  ..mount('/assets', staticFiles('web/assets'))
  ..withState(repository)
  ..fallback(singlePageApp('web/index.html'));

final server = await serve(app, InternetAddress.anyIPv4, 8080);
```

For an internet-facing application, wrap generated and handwritten modules in
one explicit policy:

```dart
final app = productionRouter(
  bodyLimit: 64 * 1024,
  maxConcurrentRequests: 64,
  maxQueuedRequests: 64,
  requestTimeout: const Duration(seconds: 10),
  onRecord: logger.info,
  onError: logger.error,
)..merge(itemRoutes());
```

The overload limit runs before routing, extraction and handlers. Timeout
cancellation is cooperative; arbitrary Dart futures cannot be forcibly stopped.

HTTP, WebSockets, server-rendered HTML, static assets, and isolate clustering,
all on one router.

## Examples

[`example/`](example/README.md) holds the handwritten, non-codegen examples.
[`examples/server_codegen`](../../examples/server_codegen/README.md) holds the
matching generated style. Both indexes link the equivalent use cases.

Both tracks are served over loopback sockets and driven with real clients,
which keeps the examples from rotting.

## Tests

The local pre-commit gate requires 100% line coverage. GitHub Actions are
reserved for Rust releases.

```bash
dart test
dart run tool/server_soak.dart 3600 32 64
```
