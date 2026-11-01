# Functional Dart migration

This contributor plan audits handwritten Dart in each publishable package. It
does not change an application-facing API by itself. Each implementation step
still needs a focused review, tests, and the complete local gate.

## Package audit status

| Package | Status |
| :--- | :--- |
| `dust_dart` | audited below |
| `dust_flutter` | pending |
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
