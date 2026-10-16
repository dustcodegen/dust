# Source-fact annotation namespace graph

Status: scoped resolver implementation for [#112](https://github.com/dustcodegen/dust/issues/112),
under the approved [namespace policy](annotation-namespaces.md) and #84.
This does **not** connect the graph to production build/discovery or enable server
generation. No Dart API, SDK floor, dependency, version or workflow changes.

## Boundary and use

`NamespaceGraph` consumes `ParsedDartFileSurface` facts, original file IDs and a
caller-owned URI-resolution callback. That callback receives the containing unit
URI and the decoded static Dart literal value. It must return the actual resolved
library identity or an error; there is no package-name, filesystem or host-platform
guess inside the graph. Raw literals and spans remain unchanged in parser facts.
Duplicate unit URIs or file IDs fail before changing graph state.

For Rust callers with a fully loaded graph:

```rust,ignore
graph.insert_library(uri, file_id, &parsed, resolve_uri)?;
let result = dust_resolver::resolve_library_in_namespace(
    file_id, source_path, output_path, &parsed, &catalog,
    graph.scope(uri), partless_config_symbols,
);
```

`resolve_annotation_ir_in_namespace` uses the same ownership decision for a single
annotation and collects source-labeled diagnostics. Neither entrypoint rewrites a
parsed name or its arguments to a synthetic canonical name. Context-free entrypoints
remain compatibility adapters; they do not claim imported-namespace correctness.

## Computation and ownership

- Local declarations shadow imported or re-exported names. Shared prefixes merge
  their actual import contributions. Ordered `show`/`hide` clauses filter each
  relevant name; repeated `show` is not flattened into a union.
- Export computation is name-specific, memoized, and cycle-aware. Repeated routes
  to the same defining declaration deduplicate; distinct declarations remain
  ambiguous. Inserting a new source clears graph memoization.
- All conditional URI alternatives are retained. Independent imports/exports
  coexist, while mutually exclusive alternatives are compared separately for
  relevant ownership. No condition is evaluated on the build host. Relevant
  missing inputs and deferred metadata are diagnosed, not silently downgraded.
- URI-form and named parts require an explicit matching owner and backlink.
  Part declarations share the owner's URI but retain their defining file/span.
  Missing parts, direct imports of parts and invalid ownership fail clearly.
- Explicit annotation origins bind a visible declaration to its canonical owner.
  Transparent named-type aliases can forward ownership. Opaque constants,
  unsupported alias facts and cyclic alias provenance stay unknown, not foreign.
  Known foreign declarations never fall back to a registered same-name owner.
- Only genuinely unknown provenance allows unique eligible legacy lookup. A
  nested derive trait considers trait eligibility for that exact alias, not a
  role registered under another alias. Overlapping eligible owners fail clearly.
  The intrinsic `Derive` marker and both roles of `Validate` are retained.

Class, enum/variant, field, constructor, method and method-parameter resolution
bind before feature normalization. Nested `Derive` constructor values retain
their full written prefixes/spans. The standalone annotation-IR entrypoint uses
the same policy. In particular, a server-owned `GET` never becomes a Dio verb.

## Evidence and remaining gates

The Rust regression suite parses the real client and server declaring libraries,
then binds both `GET` annotations in either registration order. It verifies
client HTTP normalization and the absence of that normalization for server GET,
plus exact canonical ownership, raw source/span retention, visibility, parts,
aliases, cycles, conditions, ambiguity and labeled input failures. Smaller graph
fixtures use declared absolute identities; their test callback is not a
replacement for production package-config or relative-URI resolution.

Still required under #84 before claiming production namespace support:

1. A real package-config/source loader, including implicit SDK namespaces and an
   explicit policy for the generator's own not-yet-created output parts.
2. Wire scoped binding into production resolution and every metadata-lowering
   caller: library/top-level declarations, constructor parameters and other
   currently raw member metadata. The new standalone IR API is not proof that
   those existing driver callers have been migrated.
3. Track actual reads and graph dependencies through discovery/analysis/output
   caches; changing barrels/visibility must invalidate results. Missing required
   reads must preserve previous output/cache bytes.
4. Qualifier trivia/named-constructor and lexical-shadowing audits, plus graph
   performance/conditional fan-out checks against real dependency graphs.
5. Full end-to-end server generation and minimum/latest SDK evidence using that
   production path. Existing consumer compatibility alone does not prove it.

Only local validation is allowed. All implementation PRs remain unmerged until
separately approved; the only enabled remote workflow is manual Rust release.
