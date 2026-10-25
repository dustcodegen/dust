# Server Runtime: Production Readiness

Status: `packages/dust_server` is a stable server runtime and generated wiring
surface. This document tracks the operational evidence still needed for
high-confidence, sustained production deployments; those gaps do not change
the API's stable status.

For the feature itself, see the [Server Plugin Design](server-plugin.md).

## Where it stands

100% line coverage, gated by the local pre-commit hook, and no analyzer
findings. GitHub Actions are reserved for releases. That says the code does
what its tests say. It does not say the tests ask the right questions,
which is what the rest of this document is about — and the record backs the
distinction: every defect below was found by writing an example or probing a
combination, never by the coverage number moving.

More than twenty defects have been found and fixed since, nearly all by writing
an example or probing a combination rather than by reading the code — which is
the strongest argument in this document for how the rest of it should be read.

Six of those came from one pass over deployment behaviour rather than API
behaviour, and three of the six shared a shape worth naming: a failure **after
the handler returned**, where `guard` cannot reach and the request's zone no
longer exists. A response header that could not be written hung the connection
with nothing logged; a server-sent events generator that threw never reached
`onError`; and a router factory that failed inside a spawned isolate hung
startup forever. That seam deserves a deliberate audit rather than another
round of probing.

## Covered

Router coverage is organized one folder per case under `test/router`, so a gap
is visible as a thin folder rather than hidden inside a long file:

| Folder | Cases |
| :--- | :--- |
| `matching/` | placeholders, percent encoding, literal segments, trailing slashes |
| `methods/` | HEAD fallback, 405 and `Allow`, every verb builder |
| `composition/` | nest, merge, duplicates, sealing, fallback, describe |
| `middleware/` | layer order down the tree, layer types, nested-layer scoping |
| `state/` | scoping and override, missing state |
| `lifecycle/` | body limit, error reporting |
| `paths/` | prefix normalization and joining |
| `conformance/` | parity with `shelf_router`, hand-picked and randomized |
| `integration/` | real socket, `shelf` interop, concurrency, throughput |
| `mounting/` | mount, root mount, declaration order against a catch-all |
| `hardening/` | pathological input, untrusted request fields, decoded parameters |

Beyond the router, `test/example/` serves every example over a loopback socket
and drives each with a real client — 216 tests, one group per example, so an
example that stops compiling fails the build.

Three suites test properties no status code can show:

| Suite | Property |
| :--- | :--- |
| `response/streaming_test.dart` | events reach the client as they happen, read off a raw socket by arrival time |
| `serving/traversal_test.dart` | a static handler cannot be walked out of, sent byte for byte so no client normalizes the attempt away |
| `serving/background_tasks_test.dart` | work outliving a response is drained by shutdown, and detached from the request's span |

| Area | Evidence |
| :--- | :--- |
| Route matching | agrees with `shelf_router` on 24 hand-picked paths and 1000 generated ones |
| Real HTTP | serves over a socket and is driven by a real client: methods, status, path capture, percent-decoding, 404, 405, HEAD |
| Ecosystem fit | runs behind `shelf` middleware, hosts plain `shelf` handlers, mounts inside `shelf_router`, composes with `Cascade` |
| Concurrency | 200 simultaneous requests keep their own path parameters; the matcher is immutable after build |
| Pathological input | 10000-segment paths, 100 KB segments, and a 200-route table all answer in bounded time |
| Untrusted fields | a method carrying CRLF cannot forge a response header; paths are matched literally after Dart normalizes them |
| Body limits | `content-length` is refused before reading, and streamed bodies are cut off at the limit |
| Background work | `BackgroundTasks` is drained with the requests, inside the same budget; a task that throws is reported with its name rather than taking the isolate down |
| Failure taxonomy | 400, 401 with `WWW-Authenticate`, 403, 405 with `Allow`, 413, 415, 422, 500 opaque with the detail going to `onError` |
| Validation | a failed constraint answers 422 naming every field, whether it was caught by `ValidatedExtractable` or thrown by a generated `deserialize` |
| WebSockets | binary frames, subprotocol negotiation, close codes, broadcast between connections, and an upgrade served beside HTTP on one router |
| Rendering | templates loaded from a real directory, partials resolved, interpolation escaped |

Measured, on one developer machine, worst case over a 100-route table:
**27us per match**. That is a floor to compare against, not a promise.

## Missing, in the order it matters

### 1. Sustained load is reproducible, not universal proof

`dart run tool/server_soak.dart 3600 32 64` drives the real HTTP server for an
hour, records request and handler failures, checks bounded RSS growth, and
requires graceful drain. It is deliberately local: GitHub Actions are reserved
for releases, and a shared runner is not a controlled benchmark machine.

One passing developer-machine run proves that run only. Applications still
need a soak with their database, middleware, proxy, payload distribution, and
deployment limits before calling their own service production-proven.

### 2. Timeouts support cooperative cancellation

`RequestTimeout` answers 503 past the budget and installs a
`RequestCancellation`. Handwritten handlers read it with
`requestCancellationOf(request)`; generated handlers use
`@Extract(RequestCancellationExtractable)`. A queued request that times out is
removed before its handler runs.

Dart still cannot forcibly cancel an arbitrary future. In `productionRouter`,
`ConcurrencyLimit` sits inside the timeout, so non-cooperative work keeps its
slot until it really ends rather than escaping the cap after its client receives
503. Database and network libraries must consume the signal or provide their
own deadlines to stop earlier.

### 3. Stateful middleware can be released

`DisposableLayer` gives a layer a `dispose()` that `close(drain:)` calls, after
background work has drained and whether or not there is any. A rate limiter
holding a client, a metrics flusher, a key cache on a timer: each now has
somewhere to release it, and one that throws does not stop the others.

None of the built-in layers implements it because none owns a resource.
Application layers that own a client, timer, or flusher must implement it. The
server calls disposal once after request and background-task draining, including
when the drain budget expires.

### 4. Application work is bounded; connections belong to the proxy

tower's `Service` has two methods. `call` handles a request; `poll_ready` says
whether the service can take one at all. Everything tower builds on top of that
second method — `limit`, `load_shed`, `balance` — has no counterpart here.

Dart's equivalent of a service is a function, so it cannot expose tower's
pre-call `poll_ready`. `ConcurrencyLimit` provides the enforceable boundary the
application does control: admitted work never exceeds the configured maximum,
the optional FIFO queue is bounded, and a full queue answers 503 with
`Retry-After` before routing, extraction, or handler execution.

The HTTP server has already accepted and parsed enough of the request to call
middleware. Connection count, slow headers, and acceptance-level load shedding
belong at the trusted reverse proxy. Dust does not pretend an application layer
can replace that boundary.

### 5. Multipart: buffered by default, streaming when you need it

`MultipartExtractable` holds every part in memory at once, bounded by the body
limit. That is the right trade for a form — several parts, readable in any
order — and a memory amplifier the moment the limit is raised for uploads.

`StreamedMultipartExtractable` is the answer for those: the parts arrive one at
a time and can be piped straight to disk, so nothing larger than a socket chunk
is ever held. The trade is explicit — the parts are ordered and consumable once,
so a handler needing to read part three before part one still has to buffer.

Two limits, because they stop different things. The **body** limit bounds the
whole request, checked up front against `content-length` and as the bytes flow
when there is none — a streamed upload usually declares no length. The **part**
limit bounds one file, and without it a single part can be the entire body
budget.

`example/multipart_stream.dart` shows it, including the part worth copying: the
file is stored under a generated id and the client's filename is kept as data,
because `../../etc/passwd` is a valid filename as far as a client is concerned.

### 6. Shutdown drains accepted traffic

`serve` returns a handle that counts in-flight requests, and `close` waits for
them within a deadline. The real-socket suite now starts twenty accepted
requests, begins shutdown while all are active, releases them, and requires all
twenty responses plus an empty in-flight count. The soak probe also requires a
successful bounded drain after sustained traffic.

A cluster shutdown is bounded now: `test/serving/isolates_shutdown_test.dart`
wedges a worker isolate on a synchronous loop and shows `close` still returns
inside `drain + 5s` and kills the isolate. What that test also shows is worse
than it sounds — **killing the isolate does not release the socket it bound**,
so the port stays held until the process exits. A supervisor that restarts a
wedged worker in-process will not get the port back; replacing the process is
the only recovery.

### 7. Observability has traces and logs, and metrics are the application's

`RequestId` and `AccessLog` cover the log line, `onError` covers the failure,
and `Tracing` records one span per request in W3C Trace Context, so a trace
survives a hop into a service written in something else.

Metrics stay out of the runtime on purpose: what to count, how to bucket it, and
what to call it are decisions only the application can make, and a built-in set
that does not match your dashboard is worse than none. `AccessRecord` carries
what a counter needs, `matchedRoute` included — which is the part that decides
whether the labels aggregate or explode into one series per id.
`example/metrics.dart` builds a Prometheus endpoint on it.

No exporter ships either. `SpanExporter` is one method, deliberately, so
binding to a collector stays an application's dependency rather than the
runtime's.

### 8. TLS and proxy trust are explicit

The serving guide demonstrates `SecurityContext`, recommends upstream TLS
termination for the common deployment, and deliberately does not trust
`X-Forwarded-*` automatically. It also assigns connection and header limits to
the trusted proxy rather than mislabeling handler middleware as transport-level
backpressure.

### 9. The matcher is bucketed, not a trie

Routes bucket by their whole literal prefix: 29us at 100 routes, 13us at 1000,
11us for 500 sharing one prefix, 6us for static paths. What is left is a scan
within a bucket, so routes that are identical up to their first parameter still
compete. A trie would fix that; the numbers do not yet justify it.

### 10. One core unless the application chooses isolates

`serve` serves from the isolate that called it, and a Dart isolate is one
thread. On a four-core machine that is one core busy and three idle, under any
load, with nothing in the logs or the metrics to say the ceiling is the
process rather than the application.

`serveIsolates` fixes it — the same answer `uvicorn --workers` and `gunicorn -w`
give Python, for the same reason — and requires an explicit isolate count. The
serving guide and API documentation put the one-core behavior beside the first
serving example instead of hiding it in an optimization note.

Warning about it automatically is worse than it sounds: `Platform.numberOfProcessors`
reports the host's cores, not the container's CPU limit, so the obvious check
fires loudly and wrongly on every constrained deployment. Until there is a
reliable way to read the effective limit, this stays a documentation problem.

### 11. Local validation has coverage and reproducible diagnostics

The pre-commit hook runs package analysis, tests, generated fixture checks, and
the 100% line-coverage floor locally. GitHub Actions run only for releases. The
route suite records local timing diagnostics, while the soak probe records
traffic, failures, drain state, and RSS. Neither uses a shared-host timing
threshold: a meaningful regression gate needs a named controlled machine and a
retained baseline.

### 12. Coverage is complete, and now gated

The local coverage gate requires 100% of lines. Reaching it was worth more than
the number: closing the gaps, and then hunting past them, surfaced six real
defects — each now pinned by a test.

| Found | Was |
| :--- | :--- |
| `guard` swallowed `HijackException` | every WebSocket upgrade behind a verb builder answered 500 |
| `ws()` discarded the negotiated subprotocol | `session.protocol` was always `null` |
| A killed cluster isolate keeps its bound socket | an exclusive rebind of the port fails until the process exits |
| HTML mode cached `/` as immutable | the root resolves through the default document, so the shell was pinned for a year — the exact failure HTML mode exists to prevent |
| `coerce<int>` read `0x10` as 16 | `?id=0x10` and `?id=16` named the same record, and a check written against the text saw two different strings |
| A 401 challenge could carry CRLF | an extractor building its realm from anything a client influenced was a response-splitting hole |

The floor earned its keep the hour it landed: adding the request extension
left `multipart()` and `peer()` uncovered, and the gate caught both before the
change was finished.

A covered line is still not a checked line — 100% says every line ran, not that
every branch through it was asserted.

### 13. Stable API changes require migration discipline

Earlier hardening moved `RouteGroup` to `Router`, `@Ctx` to `@State`, and
dropped OpenAPI. Stable public releases mean future breaking changes require a
versioned migration rather than an unannounced source edit. The current-source
generator is now the runtime's main consumer, and `Router` already implements
`Service`.

Argument order and the shutdown model should not follow axum. It takes a bound
listener because that is the Rust idiom, and gates shutdown on `Drop`; Dart
binds inside `serve` by convention and has no destructor, so `ServerHandle`
with a bounded `close(drain:)` is the better fit and keeps a timeout axum does
not have.

## What would make it ready

The reusable runtime work is complete when the local pre-commit gate, the
minimum/latest Dart server consumer, and the one-hour soak probe pass from the
release candidate. The application deploying it still owns:

1. capacity values measured against its real database and payloads;
2. reverse-proxy connection, header, TLS, and trusted-forwarding policy;
3. process supervision for a synchronously wedged isolate;
4. a controlled-machine benchmark baseline if throughput is a release gate.

The honest description is now "production-safe defaults with local semantic
and sustained-load evidence", not a promise that an arbitrary deployment is
production-ready without its own dependencies and limits.
