# WebSockets

## Generated typed routes and clients

Typed `@WS` routes and generated WebSocket clients are unreleased source for
planned 0.4.0. Use a matching CLI built from this checkout and its local runtime
packages. Published 0.3.1 does not include this generator or typed connection API.
The handwritten `ws(...)` and `WebSocketSession` APIs below remain available.

### Run the generated example

The example exchanges a typed chat command and acknowledgement over a real
socket. It shows a room path, a sequence query, bearer authentication before
upgrade, a negotiated protocol, and explicit connection cleanup.

| File | Responsibility |
| :--- | :--- |
| [Shared messages](../../fixtures/server_client_app/lib/websocket_contracts.dart) | `ChatCommand` and `ChatNotice`, deriving both `Serialize()` and `Deserialize()` |
| [Server controller](../../fixtures/server_client_server/lib/websocket.dart) | `@WS` route and authentication extractor |
| [Client selection](../../fixtures/server_client_server/dust.yaml) | sends the `public` group to `server_client_app` |
| [Server entrypoint](../../fixtures/server_client_server/bin/websocket_example.dart) | serves loopback port 8080 until Ctrl-C |
| [Dart client](../../fixtures/server_client_server/example/websocket_client.dart) | sends `hello`, reads the reply, and closes both transports |
| [Flutter UI](../../examples/shopping_app/lib/generated_websocket_example.dart) | connection status, message display, send button, and widget disposal |

Start from the repository root in this WS checkout, using the compatible SDK
setup in [CONTRIBUTING.md](../../CONTRIBUTING.md). Resolve the workspace and
generate shared codecs before generating the server and client:

```bash
flutter pub get
cargo run --locked -p dust_cli -- build --root fixtures/server_client_app
cargo run --locked -p dust_cli -- build --root fixtures/server_client_server
cargo run --locked -p dust_cli -- check --root fixtures/server_client_app
cargo run --locked -p dust_cli -- check --root fixtures/server_client_server
```

Terminal 1, from the repository root:

```bash
dart run fixtures/server_client_server/bin/websocket_example.dart
```

Expected startup output:

```text
Typed chat fixture: http://127.0.0.1:8080
```

Terminal 2, from the same checkout:

```bash
dart run fixtures/server_client_server/example/websocket_client.dart
```

Expected client output, followed by a clean exit:

```text
Ada in lobby: hello
```

The client also accepts a base URL as its single optional argument. Its
`fixture-token` is a local demonstration credential; replace the authentication
extractor and credential handling for an application. These commands use the
native connector because the route requires an authorization header. They do
not demonstrate browser authentication. Stop the server with Ctrl-C.

The automated proof uses an OS-assigned port instead of 8080 and runs the actual
client entrypoint as a subprocess:

```bash
dart test fixtures/server_client_server/test/websocket_loopback_test.dart fixtures/server_client_server/test/websocket_uri_test.dart
```

The tests also check two ordered replies, room encoding, protocol negotiation,
and rejection before entering a session: invalid query values or a missing
required header return 400; an invalid bearer credential returns 401.

### Author the route

Import the typed runtime explicitly with `package:dust_dart/ws.dart`. Message
direction is `WebSocketConnection<Send, Receive>` from the current endpoint's
perspective. For example, this server signature sends notices and receives
commands:

```dart
import 'package:dust_dart/ws.dart' as sockets;
import 'package:dust_server/server.dart';
import 'package:server_client_app/websocket_contracts.dart' as contracts;

part 'websocket.g.dart';

@Controller('/rooms', tags: ['public'])
class Chat with _$Chat {
  @WS('/{room}', protocols: ['dust.chat.v1'])
  Future<void> chat(
    @Path() String room,
    sockets.WebSocketConnection<contracts.ChatNotice, contracts.ChatCommand>
        socket, {
    @Query('sequence') required int sequence,
  }) async {
    await for (final command in socket.messages) {
      await socket.send(
        contracts.ChatNotice(
          room: room,
          sequence: sequence,
          user: 'example-user',
          text: command.text,
          protocol: socket.protocol,
        ),
      );
    }
  }
}
```

Use the existing [server client-package configuration](getting-started.md#generate-a-client-package-from-the-routes),
then run `dust build` from the server package. Each matching route group gets
one client; HTTP and WS endpoints share its class and barrel. A WS-only group
also produces a client. The server signature above becomes:

```dart
Future<WebSocketConnection<ChatCommand, ChatNotice>> chat(
  String room, {
  required int sequence,
});
```

The client reverses the two directions once. WS handlers return `void` or
`Future<void>` and have exactly one direct, nonnullable typed socket parameter.
A prefixed import or reexport of the canonical Dust class works; socket typedef
aliases are rejected until generic alias substitution is supported.

Put shared message models in a client-safe package and import them with a
prefix in the server library. Derive `Serialize()` and `Deserialize()` on both
models when generating both sides. Dust uses their public generated codecs;
handwritten `fromJson` look-alikes do not establish a payload contract. Each
frame is JSON text with one object root. Arrays can be model fields; top-level
arrays, scalars, null and binary frames are outside this typed wire contract.

Path, query, header, state and authentication extractors run before upgrade.
Body readers are rejected. Server-only state/custom extractors are omitted
from the client method. WS is a GET route, so a duplicate GET/WS path within
the same generated group is rejected. Applications still own mounted-router
composition, trusted origins, authentication and active-session limits.

## Typed client transport and ownership

Generated clients keep the existing `Client(Dio, {String? baseUrl})` calls
valid. A class containing WS routes additionally accepts `webSocketConnector`.
The [complete authenticated fixture](../../fixtures/server_client_server/lib/websocket.dart)
requires an explicit authorization header and uses the generated client as follows:

```dart
import 'package:dio/dio.dart';
import 'package:dust_dart/ws.dart';
import 'package:server_client_app/websocket_contracts.dart';
import 'package:server_client_app/websocket_client.dart';

Future<void> main() async {
  final dio = Dio(BaseOptions(baseUrl: 'http://127.0.0.1:8080'));
  final client = ChatClient(dio);
  final WebSocketConnection<ChatCommand, ChatNotice> socket = await client.chat(
    'lobby',
    sequence: 1,
    token: 'Bearer fixture-token', // Local fixture credential only.
  );
  try {
    final reply = socket.messages.first;
    await socket.send(const ChatCommand(text: 'hello'));
    print((await reply).text); // hello
  } finally {
    await socket.close();
    dio.close();
  }
}
```

The generator combines the optional base override with `Dio.options.baseUrl`
using the HTTP client's existing rules, then uses Dio's base-path concatenation
for the route. A leading route slash retains the base URL's path prefix. Each
path parameter is encoded once. `http` becomes `ws`, `https` becomes `wss`;
`ws` and `wss` are accepted. Unsupported schemes, missing hosts and fragments
are rejected. Query values become strings or repeated-key string lists, with
null values omitted. Base URL query values are merged first, route query values
next, and bound method query values last; later keys replace earlier keys.

The separate WS connector receives only the explicit generated handshake
headers and protocols. Dio interceptors, default headers, cookies and automatic
authentication do not configure it. The native default connector supports
headers. The browser connector rejects any nonempty headers map because browser
WebSockets cannot set arbitrary handshake headers. The authenticated fixture
above is therefore a native example. A browser endpoint needs its own deliberate
authentication design; Dust never moves a token into a query parameter.

`connectWebSocket` and generated methods return after `ready`. Adapting an
existing channel with `WebSocketConnection.fromChannel` also permits an unready
channel: `send` waits for readiness before serialization and enqueueing. Await
sequential sends when application ordering matters. Send completion means the
frame was enqueued, not acknowledged by the remote application.

`messages` is one ordered, single-subscription stream. Malformed JSON,
non-object roots, binary frames and codec failures are stream errors; later
valid frames can still arrive. Supply an error handler. Own the subscription
and explicitly close the connection; cancelling a subscription does not promise
to cancel network I/O. `done` observes the outgoing sink, and observing peer
close may require consuming messages. There is no reconnect, retry, request
correlation or background-isolate decoding.

The [runnable local server](../../fixtures/server_client_server/bin/websocket_example.dart)
serves that fixture on port 8080. The [isolated Flutter example](../../examples/shopping_app/lib/generated_websocket_example.dart)
uses the same generated client/models and closes a connection that finishes
opening after its widget has been disposed. Browser compilation is portability
evidence only; it does not prove a browser handshake or header authentication.

## Handwritten routes

An upgrade is a `GET`, so it registers like any other route and lives on the
same router as the HTTP API.

```dart
final app = Router()
  ..route(
    '/chat/{room}',
    ws(joinRoom, allowedOrigins: const {'https://app.example'}),
  )
  ..route('/health', get(health))
  ..withState(hub);

Future<void> joinRoom(WebSocketSession session) async {
  final room = session.pathParameters['room']!;
  await for (final message in session.textMessages) {
    hub.broadcast(room, message);
  }
}
```

The handshake is `shelf_web_socket`'s; this package adds the session and the
route. `allowedOrigins` is forwarded to the handshake and rejects a browser
origin outside the set before the connection is upgraded.

## The session

`WebSocketSession` keeps the `Request` that upgraded, so everything about it is
still reachable after the handshake:

| Member | Purpose |
| :--- | :--- |
| `pathParameters` | what the route captured |
| `request` | the upgrading request, for extractors such as `StateExtractable` |
| `messages` | everything, `String` or `Uint8List` |
| `textMessages` / `binaryMessages` | one kind only |
| `send` / `sendBytes` | write to the peer |
| `close([code, reason])` | end it |
| `done`, `closeCode`, `closeReason` | how it ended |
| `protocol` | the subprotocol agreed at handshake |

## Close codes

RFC 6455 reserves everything under 3000 for the protocol, and
`web_socket_channel` enforces it: an application may send 1000 or 3000-4999 and
nothing else. That rules out 1011, the code that would otherwise mean "the
server broke", so this package defines its own:

| Constant | Code | Meaning |
| :--- | :--- | :--- |
| `WebSocketClose.normal` | 1000 | the conversation finished |
| `WebSocketClose.handlerFailed` | 4500 | the handler threw |
| `WebSocketClose.goingAway` | 4501 | the server is shutting down |

A handler that throws closes with `handlerFailed` and reports to `onError`,
rather than leaving the socket open.

## What a plain GET sees

A request to a WebSocket path that is not an upgrade is answered by
`shelf_web_socket` with **404**, not 400. The route exists; the handshake did
not.

## Trying it

`dart run example/websockets.dart`. The upgrade routes sit in the same table as
everything else, and get the same layers.

The refusals come first, because they are the part that has to work:

```bash
# no ticket
curl -si --http1.1 localhost:8080/echo \
  -H 'connection: Upgrade' -H 'upgrade: websocket' \
  -H 'sec-websocket-version: 13' -H 'sec-websocket-key: dGhlIHNhbXBsZSBub25jZQ=='

# a foreign origin
curl -si --http1.1 'localhost:8080/echo?token=t-ada' \
  -H 'origin: https://evil.example' \
  -H 'connection: Upgrade' -H 'upgrade: websocket' \
  -H 'sec-websocket-version: 13' -H 'sec-websocket-key: dGhlIHNhbXBsZSBub25jZQ=='
```

```
401 {"error":"a ticket is required"}
403 (upgrade rejected: invalid origin)
```

Both are checked at the upgrade, while it is still HTTP and a status code can
still be sent. After the handshake there is no status code left to send.

**The `Origin` check is not optional for browser sessions.** The same-origin
policy does not apply to WebSockets: any page on the internet may open one to
your server carrying the user's cookies. Pass the trusted browser origins to
`ws(allowedOrigins: ...)`. An absent origin remains allowed for non-browser
clients; require it in an upgrade guard when an endpoint is browser-only.
`ConcurrencyLimit` covers only the HTTP upgrade. Bound active WebSocket
connections per identity in the application or at the trusted reverse proxy.

The credential rides in the query string because a browser cannot set headers on
a WebSocket handshake. It therefore lands in access logs — so use a short-lived
ticket fetched over HTTPS, not the session token.

With both in place the handshake succeeds:

```bash
curl -si --http1.1 'localhost:8080/echo?token=t-ada' \
  -H 'origin: http://localhost:3000' \
  -H 'connection: Upgrade' -H 'upgrade: websocket' \
  -H 'sec-websocket-version: 13' -H 'sec-websocket-key: dGhlIHNhbXBsZSBub25jZQ=='
```

That answers `101 Switching Protocols`. For an actual conversation use a
WebSocket client — `websocat 'ws://localhost:8080/echo?token=t-ada'` with an
`Origin` header — and every message comes back prefixed with `echo:`.

`/greeter` shows subprotocol negotiation: `protocols` is offered in preference
order, the client picks, and the result is `session.protocol`. It is how one
endpoint serves two client versions without a second URL.
