# Functional Dart migration

This contributor plan audits handwritten Dart in each publishable package. It
does not change an application-facing API by itself. Each implementation step
still needs a focused review, tests, and the complete local gate.

## Package audit status

| Package | Status |
| :--- | :--- |
| `dust_dart` | audited below |
| `dust_flutter` | audited below |
| `dust_db_sqlite3` | audited below |
| `dust_db_postgres` | audited below |
| `dust_server` | pending |

## Direction for new code

- Return `Result<T, E>` for an expected operation failure that a caller can
  inspect or recover from.
- Return `Option<T>` for domain absence when `null` is not itself a meaningful
  value.
- Keep `null` at Dart, Flutter, JSON, SQL, and third-party boundaries where the
  external API uses it. Convert it once when it enters Dust-owned logic.
- Prefer exhaustive `switch` patterns for `Result`, `Option`, and other sealed
  states. Use `map`, `mapErr`, `andThen`, and `orElse` when they make the data
  flow shorter and clearer.
- Keep `if` for a plain boolean guard when a pattern or combinator would obscure
  the condition.
- Catch exceptions only where Dust must translate an SDK, driver, framework, or
  user callback failure. Convert recoverable failures to a typed error at that
  boundary; do not catch an error only to discard it.
- Do not replace a stable nullable or throwing API in place. Add and prove the
  typed path first, migrate Dust-owned callers and generated output, then review
  deprecation or removal for a major release.

## `dust_dart`

The functional core already uses sealed `Option` and `Result` types, exhaustive
patterns, and focused combinator extensions. The remaining exception and
nullable paths are concentrated in JSON conversion, database row boundaries,
and optional error metadata.

### Planned migrations

| Priority | Area | Plan | Compatibility proof |
| :--- | :--- | :--- | :--- |
| 1 | `JsonHelper.asMap` | Replace exception-driven map-shape detection with a typed map pattern while preserving the same key-labelled `ArgumentError`. | Existing JSON helper tests plus wrong-key and nested-map cases. |
| 1 | JSON decoding | Introduce a typed JSON decode error and result-returning helpers. Keep `fromJson`, `JsonHelper` throwing methods, and `decodeJsonObject` as compatibility adapters over the typed path. | Exact SerDe snapshots, malformed scalar/list/map/codec tests, and analyzer-clean generated fixtures. |
| 2 | Row absence | Add `Option`-returning row readers alongside `readNullable`, then migrate Dust-generated row mapping and Dust-owned examples before considering removal of nullable aliases in a major release. | Both database drivers, generated row snapshots, SDK-consumer fixtures, and nullable-column examples. |
| 2 | Custom codecs | Add a result-returning codec path so an expected decode failure does not need to throw. Keep exception translation around existing `Deserializer` implementations until callers migrate. | Codec success, typed failure, thrown user-code failure, and nested JSON-path tests. |

### Deliberate boundaries

- `Option.fromNullable` and `Option.toNullable` remain the explicit conversion
  points for Dart APIs.
- SQL `NULL`, JSON `null`, nullable model fields, and nullable annotation
  options remain nullable at their data boundary; they are not automatically
  failures.
- `SqlxError.driver`, `operation`, `cause`, and `kind` stay nullable public
  metadata for compatibility. Dust-owned control flow should pattern-match or
  convert them instead of using forced null assertions.
- `inspect` and `inspectErr` keep their small conditional side effects; a
  combinator rewrite would not make them clearer.

### Implementation order

1. Add typed JSON errors and result-returning helpers without changing generated
   output.
2. Move existing throwing JSON entry points onto those helpers and verify exact
   exception compatibility.
3. Add `Option` row readers to the shared interface and both driver adapters.
4. Update the database emitter, snapshots, fixtures, and examples to use the
   typed readers.
5. Add the result-returning custom codec path, then migrate generated decoding.
6. Review old adapters only during an explicitly approved major-release change.

## `dust_flutter`

The runtime already models async UI state with sealed classes and uses pattern
matching in generated-facing code. The main nullable sentinels are in routing:
route parsers use `null` for both absence and invalid input, while guards use
`null` to mean "allow." The async ViewModel example also turns `Err` into a
thrown object so the base class can catch it again.

Using `Result` or `Option` in this package's public API requires moving
`dust_dart` from a development dependency to a runtime dependency. That change
must land with a standalone Flutter consumer check; generated code must not
rely on an undeclared transitive dependency.

### Planned migrations

| Priority | Area | Plan | Compatibility proof |
| :--- | :--- | :--- | :--- |
| 1 | Route parameters | Add typed parsers that return `Result<Option<T>, RouteParameterError>` so a missing optional value and malformed input are distinct. Move generated path and query decoding to the typed helpers; keep the nullable `generatedRouteParse*` functions as adapters. | Helper unit tests, route parser edge cases, exact route snapshots, deep-link restoration, and analyzer-clean generated fixtures. |
| 1 | Guards and redirects | Add `Option<T>`-returning guard and redirect contracts, then adapt the existing `T?` contracts at one boundary. Make the internal guard chain always return `Future<Option<T>>` instead of using both a nullable future and a nullable result. | Guard order, sync and async redirects, auth-protected routes, redirect cycles, stale navigation, and browser restoration tests. |
| 1 | Async actions and loading | Add a result-native path that accepts `Result<T, E>`, preserves action-token stale-result suppression, and maps `Ok`/`Err` directly to callbacks or `AsyncState`. Keep throwing `runAction` and `loadData` entry points as compatibility adapters. | Action-token tests, nullable data, refresh with previous data, typed errors, retries, disposal, and the async ViewModel example. |
| 2 | Navigation completion | Return `Option<_RouteEntry<T>>` from the private navigation pipeline for disposed or superseded work. Keep Flutter's public `Future<R?>` push result because `null` is a valid Navigator pop result. | Push/pop result, concurrent navigation, disposal, refresh, replace, and branch-stack tests. |
| 2 | i18n parsing | Add a typed ARB parse result and keep `I18nArbParser.parse` as the throwing compatibility adapter. Catch `jsonDecode` failures only at this boundary and preserve asset-loading failures from Flutter. | Malformed JSON, non-object JSON, metadata filtering, locale fallback, overrides, and asset discovery tests. |
| 2 | Routing invariants | Remove avoidable null assertions when composing URI fragments, restoring stacks, and resolving `RouterScope`; use patterns or explicit Flutter errors while preserving the same successful behavior. | URI round trips, restore mode, missing-scope diagnostics, and release-mode widget tests. |

### Deliberate boundaries

- Flutter `Page` fields, `BuildContext.maybeOf`, route branch metadata, and
  Navigator pop results stay nullable because Flutter defines those contracts.
- `AsyncState<T>` remains the authoritative sealed lifecycle. Its nullable
  convenience getters stay for widget ergonomics and to preserve the existing
  distinction between no data and present nullable data.
- Initialization keeps `try`/`finally` so `_initFuture` is cleared after both
  success and failure. Diagnostic route formatting keeps its fallback catch so
  logging cannot break navigation.
- Asset bundle I/O, user callbacks, and legacy throwing overrides remain
  exception boundaries. New Dust-owned flow converts expected failures once
  and pattern-matches them afterward.
- Plain lifecycle guards, equality checks, and collection loops keep their
  direct `if` statements; replacing them with combinators would be less clear.

### Implementation order

1. Promote `dust_dart` to a declared runtime dependency and add a standalone
   consumer fixture before exposing its functional types.
2. Add typed route parameter helpers and migrate the route emitter, snapshots,
   runtime tests, and generated examples.
3. Add typed guard and redirect contracts, bridge legacy contracts once, then
   migrate generated routers and the routing guides.
4. Add result-native action and async loading paths, then migrate the example
   without throwing `Err` values.
5. Convert the private navigation result and remove the related null assertions.
6. Add typed ARB parsing while retaining the existing throwing entry point.
7. Review nullable and throwing adapters only during an explicitly approved
   major-release change.

## `dust_db_sqlite3`

Queries, statements, transactions, and close operations already return
`Result`, and the driver translates native SQLite failures at focused
boundaries. The remaining exception-first flow is mainly connection startup
and migrations. Internal transaction control also returns nullable errors, and
the isolate protocol relies on casts and null assertions before a catch-all
turns malformed messages into a connection error.

### Planned migrations

| Priority | Area | Plan | Compatibility proof |
| :--- | :--- | :--- | :--- |
| 1 | Connection startup | Add result-returning foreground and worker `open`/`connect` paths, including generated database facade entry points. Move option validation, native open, pragmas, and migration setup onto that core; keep the current throwing factories and generated methods as adapters. | Connection-option validation, read-only and memory modes, native open failures, worker startup/exit tests, exact database snapshots, and SDK-consumer fixtures. |
| 1 | Optional queries | After the shared `dust_dart` `Executor` contract adds an `Option` path, implement `Result<Option<T>, SqlxError>` for optional rows and scalars. Preserve `Some<T?>(null)` for a present SQL `NULL`; keep existing `Result<T?, SqlxError>` terminals as adapters. | Zero-row, present-null, wrong-type, too-many-row, foreground/worker parity, generated DAO snapshots, and both database drivers. |
| 1 | Migration execution | Make migration steps return `Result<Unit, SqlxError>` and compose them instead of throwing between Dust-owned helpers. Catch only calls into `package:sqlite3`, and preserve rollback-error precedence, the commit filter, checksum provenance, locking, and disk-full behavior. | Migration integrity, legacy baseline, locking, transaction-boundary, commit/rollback failure, disk-full, and worker migration tests. |
| 2 | Transaction control | Replace nullable-error helpers such as connection access, `BEGIN`/`COMMIT`/`ROLLBACK`, and savepoint cleanup with `Result<Unit, SqlxError>`. Represent stored recovery or aborted-transaction failures with `Option`, then use exhaustive patterns while retaining callback cleanup in `finally`. | Root/nested transaction, ownership, unawaited child, aborted transaction, failed cleanup, commit recovery, and worker parity tests. |
| 2 | Worker protocol | Decode startup arguments, commands, replies, and error payloads with typed `Result` helpers and list/map patterns. Remove protocol null assertions; convert one decode failure into the existing terminal connection error and complete every pending request once. | Invalid reply, early exit, send failure, startup failure, callback failure, close, and pending-request completion tests. |
| 2 | Row decoding | Implement the shared `Option` row readers and replace exception-driven `DateTime.parse` handling with `DateTime.tryParse` plus a typed decode error. Keep mapper exception translation at the driver boundary. | Nullable and missing columns, present-null values, bool/date coercion, mapper failures, scalar reads, and generated row mapping tests. |

### Deliberate boundaries

- `package:sqlite3`, isolate spawn/send, and user row mappers or transaction
  callbacks can throw. Their focused catches remain and immediately translate
  into `SqlxError`; transaction and statement cleanup keeps `finally`.
- Nullable connection options mean "not configured." A nullable migration
  bundle also distinguishes unmanaged startup from a managed empty bundle.
  Those stable Dart entry points remain nullable and convert once internally.
- SQL `NULL` stays nullable at the native row boundary. Typed optional-query
  paths must preserve the difference between no row and a row containing null.
- Map and statement-cache misses stay nullable at the Dart collection boundary.
  The placeholder scanner's nullable match helpers stay on its per-character
  hot path unless a benchmark proves an `Option` rewrite has no regression.
- `_workerDiagnostic` keeps its defensive catch because even an error object's
  `toString` may throw, and diagnostics must not terminate the worker.
- Cardinality checks, lifecycle guards, bounded loops, and SQL text scanning
  keep direct `if` statements where they express the condition most clearly.

### Implementation order

1. Add shared `Option` row and optional-query contracts in `dust_dart`, with
   compatibility adapters and exact generator snapshots.
2. Add result-returning direct and worker startup APIs without changing current
   callers, then expose matching generated database facade methods.
3. Move migration setup and execution onto the result-returning startup core.
4. Convert connection and transaction-control sentinels to `Result`/`Option`.
5. Add typed worker protocol decoders and remove protocol null assertions.
6. Migrate row decoding and Dust-owned examples to the typed paths.
7. Review throwing and nullable adapters only during an explicitly approved
   major-release change.

## `dust_db_postgres`

Queries, migrations, transactions, and close operations already report
`Result`, and failures from `package:postgres` are translated at the driver
boundary. The remaining exception-first setup is connection URL and pool-option
validation. Optional queries still use nullable sentinels, indexed scalar casts
can escape as `TypeError`, and savepoint cleanup currently discards its own
failure.

### Planned migrations

| Priority | Area | Plan | Compatibility proof |
| :--- | :--- | :--- | :--- |
| 1 | Connection setup | Add a result-returning connection path that validates the URL, `sslmode`, and pool size before constructing the pool. Expose a matching generated database facade entry point; keep `PostgresDriver.connect`, `PgPool`, and generated throwing factories as adapters. | Supported and rejected TLS modes, malformed URLs, option precedence, pool-size validation, exact database snapshots, and the PostgreSQL consumer fixture. |
| 1 | Scalar and row decoding | Replace unchecked indexed casts with type patterns that return `SqlxError.decode`, including nullable scalars. Keep user mapper catches at the query boundary so no expected decode failure escapes a `Result`. | Wrong-type and null scalars, missing indexes, named/indexed parity, user mapper throws, and unchecked SQL mapping tests. |
| 1 | Optional queries | Implement the shared `Result<Option<T>, SqlxError>` row and scalar terminals after the `dust_dart` contract lands. Preserve `Some<T?>(null)` for a present SQL `NULL`; keep current nullable terminals and generated methods as adapters. | Zero-row, present-null, wrong-type, too-many-row, generated DAO snapshots, query-parity fixtures, and SQLite/PostgreSQL parity tests. |
| 2 | Savepoint control | Make savepoint creation and completion return typed results, then combine callback and cleanup outcomes with explicit error precedence. Keep the private rollback signal only where `package:postgres` requires a throw to make `runTx` roll back an `Err`. | Root and nested `Ok`, `Err`, and thrown callbacks; aborted transactions; savepoint create/release/rollback failures; and multi-level nesting. |
| 2 | Connection and cache cleanup | Compose statement-cache disposal with pool close through a typed result while preserving the rule that an already-closed connection must not hide the primary close outcome. Keep stale-plan retry limited to PostgreSQL's specific server error. | Closed connections, statement eviction, retired connections, stale-plan recovery, concurrent queries, and repeated close tests. |

### Deliberate boundaries

- `package:postgres` query, transaction, TLS, and connection calls can throw.
  Their focused catches remain and translate failures into `SqlxError` once.
- `_RollbackSignal` remains a private adapter because the driver commits a
  callback that returns normally; Dust must throw an `Err` through that API to
  request rollback.
- Nullable connection options mean "use the URL or driver default." SQL `NULL`
  and nullable error metadata also stay nullable at their external boundary and
  convert once inside Dust-owned flow.
- Statement-cache and column-index map misses stay nullable collection lookups.
  Cache capacity checks, cardinality guards, and migration loops keep direct
  `if` statements because they express plain conditions more clearly.
- Migration execution is already result-native. Its driver catch and early
  `Err` patterns remain; replacing them with a longer async combinator chain
  would obscure transaction order and rollback behavior.

### Implementation order

1. Fix indexed scalar decoding so every wrong type is a typed decode failure.
2. Add the result-returning connection core and generated facade entry point,
   then migrate Dust-owned examples and fixtures.
3. Land the shared `Option` query contract and update both database drivers,
   generated terminals, snapshots, and examples together.
4. Return typed savepoint outcomes and prove cleanup-error precedence.
5. Compose cache disposal and pool close without weakening stale-plan recovery.
6. Review throwing and nullable adapters only during an explicitly approved
   major-release change.
