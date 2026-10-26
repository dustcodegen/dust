# Middleware

A layer wraps everything below the router it is added to.

```dart
final app = Router()
  ..layer(const RequestTimeout(Duration(seconds: 30)))
  ..layer(const RequestId())
  ..layer(AccessLog(logger.info))
  ..nest('/api', apiRoutes);
```

## Order

Layers run outermost first, in the order they were added, down the tree: a layer
on the root runs before one on a nested router, which runs before one on the
module it holds. A layer on a nested router never runs for a sibling branch.

The root's layers wrap the router itself, so they also cover requests that
matched nothing.

## What ships

Nothing is installed by default.

| Layer | Purpose |
| :--- | :--- |
| `RequestTimeout(budget, {onTimeout})` | answers 503 past the budget |
| `ConcurrencyLimit(maxConcurrentRequests:, maxQueuedRequests:)` | bounds executing and queued work, then answers 503 with `Retry-After` |
| `RateLimit(maxRequests:, window:, keyBy:)` | token-bucket limit per trusted application identity, then answers 429 |
| `RequestId({header, generate})` | keeps an inbound `x-request-id` or assigns a UUID, and echoes it |
| `AccessLog(onRecord)` | reports method, path, status, duration, and request id |

`AccessLog` prints nothing itself. What a log line looks like, and where it goes,
is the application's decision.

`RequestTimeout` bounds how long a handler may take, which the body limit does
not. Dart cannot forcibly cancel an arbitrary future, so work that ignores
cancellation keeps running. Cooperative handlers can read
`RequestCancellation` with `requestCancellationOf(request)` or generate it as
`@Extract(RequestCancellationExtractable)`. In `productionRouter`, concurrency
sits inside the timeout: non-cooperative work keeps its slot until it really
ends, and a queued request whose deadline passes never enters its handler.

It also bounds *producing* the response, not sending it. A handler that returns
straight away with a streamed body — an event stream, a large download — has
already met the budget, and the stream then runs for as long as it likes. Put
this layer above an SSE endpoint and it protects nothing there. Bound those in
the stream itself, with `take`, a timeout on the source, or a keep-alive the
client has to answer. `ConcurrencyLimit` keeps its slot until an unknown-length
response stream ends or is cancelled, so abandoned streams cannot bypass the
capacity limit.

## Trying it

One example per layer, each under 60 lines. The two worth running first are the
ones where placement changes the answer.

**`routeLayer` versus `layer`** — `dart run example/route_layer.dart`:

```bash
curl -si localhost:8080/admin/orders
curl -s  localhost:8080/admin/orders -H 'authorization: Bearer staff'
curl -si localhost:8080/admin/typo
curl -s  localhost:8080/health
```

```
401 WWW-Authenticate: Bearer   {"error":"expected a bearer token"}
200                            ["order-1"]
404                            {"error":"no route for /admin/typo"}
200                            {"status":"ok"}
```

The third line is the point. A guard added with `layer` answers **401** there, so
a typo in your own route table looks like an auth failure — and you spend the
afternoon on the credential instead of the spelling.

**A nested `layer` is scoped to its prefix** —
`dart run example/normalize_path.dart`:

```bash
curl -s localhost:8080/notes/      # rewritten, one response, same URL
curl -s localhost:8080/api/notes/  # nested, covered from above
```

A `layer` covers everything its router answers — the 404s and 405s inside its
prefix included — so it can run *before* routing, which is what a path rewrite
depends on. `buildScopedApp` in that example puts the layer on `/api` alone:

```
GET /api/notes/  -> 200 ["first"]
GET /shop/notes/ -> 404 {"error":"no route for /shop/notes/"}
```

Scoping earns its keep when one half of an application has URLs you must not
rewrite — a webhook whose signature is computed over the exact path, say.

**An extractor as a layer.** `fromExtractor` runs one extractor for every route
below it and passes the value on; `Extension<T>` reads it back in a handler.
Both names are axum's — `middleware::from_extractor` and `Extension<T>` — because
this is the same idea and a second vocabulary would help nobody.

```dart
final admin = Router()
  ..routeLayer(fromExtractor(const RequireScope('admin')))
  ..route('/orders', get(listOrders))
  ..route('/whoami', get(whoAmI));
```

```dart
Future<Map<String, Object?>> whoAmI(Request request) async {
  final caller = await request.extract(const Extension<Caller>());

  return {'id': caller.id};
}
```

Two things this buys over naming the extractor in every handler: the work
happens **once per request** rather than once per handler that wants the value,
and a route added to that module later cannot forget it.

A missing value is a **500**, not a 401 — the layer not being installed is a
wiring mistake in the route table, and a 401 would send whoever is debugging it
after a credential that was never the problem. Pair it with `routeLayer` rather
than `layer`, so a path that does not exist still answers 404.

**The rest**, each with its own example:

| Layer | Example | The bit worth reading |
| :--- | :--- | :--- |
| `Cors` | `cors.dart` | a preflight answers 204 without reaching a handler; `Vary: Origin` is set; credentials with `any()` throws at construction |
| `Compression` | `compression.dart` | explicit `gzip` overrides `*`; malformed or zero quality refuses it; under 1 KB is left alone |
| `RequestId` | `request_id.dart` | a client-supplied id is kept, so one id spans every hop |
| `AccessLog` | `access_log.dart` | records the 404 too, and the recorded path carries no query string |
| `SecurityHeaders` | `security_headers.dart` | CSP and HSTS have no defaults, and why; two nested routers scope one policy to each half |
| `RequestTimeout` | `request_timeout.dart` | the 503 goes back while the handler keeps running |
| `ConcurrencyLimit` | `concurrency_limit.dart` | one request executes, one waits, and the next receives 503 with `Retry-After` |
| `RateLimit` | `rate_limit.dart` | one API key receives 429 without spending another key's quota |

```bash
curl -sI localhost:8080/rows -H 'accept-encoding: gzip'      # content-encoding: gzip
curl -sI localhost:8080/rows -H 'accept-encoding: gzip;q=0'  # no encoding
curl -si localhost:8080/slow                                 # 503 after ~200ms
```

`AccessLog` is the one to point somewhere real. It hands you an `AccessRecord`
rather than printing, so a service that wants JSON on stdout, a counter, and a
sampled trace does not have to fork the layer to get them.

## Production policy

Use one outer router for handwritten and generated modules:

```dart
final app = productionRouter(
  bodyLimit: 64 * 1024,
  maxConcurrentRequests: 64,
  maxQueuedRequests: 64,
  requestTimeout: const Duration(seconds: 10),
  onRecord: logger.info,
  onError: logger.error,
)
  ..merge(generatedRoutes())
  ..nest('/manual', handwrittenRoutes);
```

The function installs request ids, access records, security headers, timeout
cancellation, and bounded concurrency. Its capacity, body, and deadline values
are required because a database-backed API and a streaming service do not have
the same safe limits. CORS, compression, CSP, HSTS, tracing, and authentication
remain application policy.

This is application-level admission: the request has already reached Dart, but
routing, extraction and handler work have not run. Bound connections and header
parsing at the trusted reverse proxy when overload must be rejected earlier.

Add a per-client rate limit separately because only the application knows what
identity is trustworthy:

```dart
String clientKey(Request request) => request.headers['x-api-key'] ?? 'anonymous';

app.layer(const RateLimit(
  maxRequests: 60,
  window: Duration(minutes: 1),
  keyBy: clientKey,
));
```

Do not key directly from `x-forwarded-for` unless a trusted proxy strips and
rebuilds it. `maxKeys` bounds memory; once full, unknown identities fail closed
with 429 rather than evicting a bucket and giving an attacker a fresh quota.

## Writing one

Two forms are accepted. A bare `shelf` `Middleware`:

```dart
app.layer(logRequests());
```

Or a `Layer`, which is const-expressible so a generated annotation can carry it:

```dart
final class RequireHeader implements Layer {
  const RequireHeader(this.name);

  final String name;

  @override
  Middleware toMiddleware() => ...;
}
```

Anything else is refused when the handler is built, naming the offending type.
