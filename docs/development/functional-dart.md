# Functional Dart migration

This contributor plan audits handwritten Dart in each publishable package. It
does not change an application-facing API by itself. Each implementation step
still needs a focused review, tests, and the complete local gate.

## Package audit status

| Package | Status |
| :--- | :--- |
| `dust_dart` | audited below |
| `dust_flutter` | audited below |
| `dust_db_sqlite3` | pending |
| `dust_db_postgres` | pending |
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
