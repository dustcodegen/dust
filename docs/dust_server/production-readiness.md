# Server Production Readiness

Status: `dust_server` and generated server wiring are stable APIs. Production
readiness still belongs to the application and its deployment: repository tests
cannot validate an application's database, proxy, traffic, secrets, or capacity.

Use the [server guides](README.md) for the shipped API. This page
records the evidence Dust requires and the limits a deployment must account for.

## Repository evidence

Run evidence from the source under review. Do not copy test totals or benchmark
timings into documentation; both change as the suite and machine change.

| Evidence | Command | What it establishes |
| :--- | :--- | :--- |
| Analyzer | `dart analyze --fatal-infos packages/dust_server` | the current package has no analyzer findings |
| Runtime suite | `dart test packages/dust_server` | router, extraction, response, middleware, serving, WebSocket, rendering, tracing, and example contracts |
| Line coverage | `bash scripts/dart/coverage.sh packages/dust_server 100` | every executable runtime line is exercised; it does not prove every behavior is asserted |
| Generated consumers | `./scripts/test.sh` | fresh server generation/check plus analyzer and request tests for the checked-in consumers |
| SDK compatibility | follow [SDK validation](../usage/sdk-validation.md) | fresh generation and execution on the declared minimum and latest local Dart SDKs |
| Sustained load | `cd packages/dust_server && dart run tool/server_soak.dart 3600 32 64` | one-hour local traffic, bounded RSS growth, no request/handler failures, and graceful drain |
| Complete local gate | `./.githooks/pre-commit` | lint, security, compatibility, tests, coverage, and generated-output checks |

GitHub Actions are reserved for release workflows. Ordinary development and
release-candidate evidence is collected locally; a missing remote check is not
evidence that these commands ran.

The suite includes differential route checks against `shelf_router`, randomized
paths, real loopback HTTP, concurrent requests, pathological input, traversal
attempts, streaming responses, WebSockets, generated handlers, and shutdown.
See [Testing](testing.md) for the test layout.

## Shipped production controls

| Concern | Dust supplies | Guide |
| :--- | :--- | :--- |
| Request size | header and streamed-body limits | [Extraction](extraction.md) |
| Application capacity | bounded concurrency and optional bounded FIFO queue | [Middleware](middleware.md) |
| Request deadlines | timeout responses plus cooperative cancellation | [Middleware](middleware.md) |
| Abuse limits | per-identity rate limiting and `Retry-After` | [Security](security.md) |
| Failure handling | typed rejections, opaque 500 responses, isolated observers | [Responses](responses.md) |
| Shutdown | bounded request, background-task, and layer disposal drains | [Serving](serving.md) |
| Transport security | `SecurityContext` support and explicit proxy trust | [Serving](serving.md) |
| Browser upgrades | explicit WebSocket origin allow-lists | [WebSockets](websockets.md) |
| Observability | request ids, access records, W3C trace context, exporter seam | [Tracing](tracing.md) |
| Safer defaults | `productionRouter` composes body, capacity, timeout, request-id, logging, and security-header policy | [Security](security.md) |

These controls apply equally to handwritten and generated routes because both
produce the same `Router` and `Handler` contracts.

## Operational limits

### Cancellation is cooperative

`RequestTimeout` can answer the client and signal `RequestCancellation`; it
cannot stop an arbitrary Dart `Future`. Database and network calls must consume
the signal or use their own deadlines. Timed-out work keeps its concurrency slot
until it actually finishes.

### The reverse proxy owns transport limits

Middleware runs after the HTTP server has accepted and parsed a request. Bound
open connections, slow headers, trusted forwarding, and edge TLS at a trusted
proxy or load balancer. Dust does not automatically trust `X-Forwarded-*`.

### Isolates are explicit process architecture

`serve` uses one isolate and therefore one core. `serveIsolates` uses several
cores, but memory is not shared: caches, counters, sessions, and connection
pools are per isolate unless they live outside the process.

Worker loss is observable through `ServerIsolates.alive` and
`onIsolateError`. A synchronously wedged isolate can keep its shared socket
bound after it is killed, so recovery is process replacement, not an in-process
worker restart.

### Memory and durable work remain application choices

Buffered multipart extraction holds the bounded request in memory;
`StreamedMultipartExtractable` is the upload path for large files.
`BackgroundTasks` participates in graceful drain but is in-process and
unpersisted. Work that must survive a crash needs an outbox or external queue.

### Metrics and exporters are application dependencies

Dust exposes access records, matched-route metadata, spans, and a
`SpanExporter` interface. It does not choose metric names, histogram buckets,
collector libraries, retention, or alert policy. The metrics example shows the
intended seam without adding an exporter dependency to the runtime.

### Package evidence is not application evidence

The soak probe deliberately uses a small built-in handler. Before deployment,
repeat sustained load with the real database, middleware, payload distribution,
proxy, and resource limits. Test backup/restore, authorization, migrations,
secret rotation, and dependency failure modes separately.

## Release-candidate gate

Before releasing `dust_server` or server code generation:

1. run `./.githooks/pre-commit` from a clean candidate commit;
2. run the explicit minimum/latest SDK matrix and retain its evidence directory;
3. run the one-hour soak probe on a named machine and retain its JSON result;
4. confirm generated consumer output is unchanged or intentionally reviewed;
5. review dependency/security audit output without adding exceptions or lowering
   thresholds.

Before deploying an application:

1. choose measured body, concurrency, queue, rate, and timeout limits;
2. configure proxy connection/header/TLS/trusted-forwarding policy;
3. run sustained load and failure tests against the real application;
4. configure process supervision for isolate or process failure;
5. configure durable logs, metrics, traces, alerts, backups, and restore tests.

The accurate claim is: Dust provides stable server APIs, production-oriented
controls, and reproducible local evidence. It does not certify an arbitrary
application or deployment as production-ready.
