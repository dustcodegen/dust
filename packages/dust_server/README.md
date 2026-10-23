# dust_server

[![pub package](https://img.shields.io/pub/v/dust_server.svg)](https://pub.dev/packages/dust_server)
[![pub points](https://img.shields.io/pub/points/dust_server)](https://pub.dev/packages/dust_server/score)
[![downloads](https://img.shields.io/pub/dm/dust_server)](https://pub.dev/packages/dust_server)

Runtime support for Dust-generated Dart HTTP servers on `shelf`.

> [!IMPORTANT]
> **0.1.0-beta.2**. Complete enough to build on; the API may still change
> before 1.0, so pin the exact version.
>
> Dust server generation is available for Phase 1 controllers and top-level
> handlers: HTTP verbs, path/query/header extraction, JSON bodies, and
> JSON/`void`/`Response` results. Custom extractors, state, raw/form/multipart
> bodies, middleware emission, and typed `Result` responses remain staged work.
> See the [design phases](../../docs/design/server-plugin.md#phases).

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

Generated controller wiring:

```dart
part 'todo_controller.g.dart';

@Controller('/todos')
final class TodoController with _$TodoController {
  @GET('/{id}')
  Future<Map<String, Object?>> read(@Path() int id) async => {'id': id};
}
```

Run `dust build`, then mount `TodoController().routes()` in the application
router. The complete generated GET/POST fixture is
[`fixtures/server_codegen_app`](../../fixtures/server_codegen_app).

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

HTTP, WebSockets, server-rendered HTML, static assets, and isolate clustering,
all on one router.

## Examples

[`example/`](example/README.md) holds one file per question — under 60 lines
each, named after the question it answers, so the answer is not buried in an
application. The index lists them by area.

Every one is served over a loopback socket by `test/example/` and driven with a
real client, which is what keeps a directory that size from rotting.

## Tests

The historical coverage gate targets 100% line coverage. Dart coverage CI is
disabled during the repository migration; only manual Rust release CI is enabled.

```bash
dart test
```
