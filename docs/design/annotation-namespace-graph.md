# Source-fact annotation namespace graph

Status: scoped resolver implementation for [#112](https://github.com/dustcodegen/dust/issues/112),
with shared metadata lowering in [#117](https://github.com/dustcodegen/dust/issues/117),
under the approved [namespace policy](annotation-namespaces.md) and #84.
Production driver integration is in progress under #114: build/check, route
inspection and watch now load scoped sources, but the full acceptance gates are
not green. This does not enable server generation. No Dart API, SDK floor,
dependency, version or workflow changes.

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

`BindingContext::new(&catalog, Some(graph.scope(uri)))` now lets the driver share
that exact context between `resolve_library_with_bindings` and IR lowering.
`None` is an explicit compatibility mode, never a fallback for a failed source
read or graph lookup. Scoped resolver errors stop before metadata lowering;
lowering ownership errors prevent IR from reaching emission.

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

Driver lowering also uses that context for retained library, mixin, extension,
extension-type, function, variable and typedef metadata. Top-level function
parameters reuse method-parameter resolution and retain typed trait/config roles.
Constructor-parameter and mixin-field ownership is validated even on otherwise
unannotated types. Their existing IR representation and mixin field normalization
stay unchanged; this does not add new annotation slots or enable inherited mixin
serialization. Nested standalone `Derive` values keep their prefixes and get
ownership validation only when the group is the actual intrinsic declaration.
Ignored groups on methods and parameters still validate nested ownership; valid
groups at those unsupported sites do not enable derivation.

Outer annotation qualifiers use AST identifier components for semantic lookup,
not the raw source slice ([#119](https://github.com/dustcodegen/dust/issues/119)).
Whitespace and comments between identifiers/dots therefore do not change their
owner. The exact written qualified name, arguments and span remain in parser
facts and raw IR; failure labels retain that source spelling. This also fixes
context-free prefix validation without changing its unique legacy lookup. It
does not yet resolve named-constructor ambiguity or change nested constructor-
value extraction; those and lexical-shadowing audits remain production gates.
Traditional/primary generic-name consumer fixtures also exercise qualifier trivia
through cold build, check, warm cache and exact generated snapshots. The imported
generic model in the product showcase retains it as a real example; minimum/latest
SDK gates compile and execute fresh generation rather than trusting copied output.

These are tests of retained parser surfaces, not a claim to full Dart metadata
extraction. Missing mixin/extension member facts and the unapproved #92 design
remain separate limitations; this change does not bypass that design boundary.

The legacy pre-parser candidate scan advances across qualifier whitespace, line
comments and nested block comments ([#120](https://github.com/dustcodegen/dust/issues/120)).
It neither strips source text nor assigns ownership. This fixes candidate loss
before parsing; production import/alias reachability and dependency fingerprinting
are handled by the separate #114 loader described below.

## Evidence and remaining gates

### Approved own-output boundary for #114

Dust's own generated output must not supply annotation declaration provenance,
whether that file is missing, current or stale. An annotation available only from
that output must fail clearly at its use, rather than enabling short-name fallback
or multi-pass generation. The user approved this boundary during #114 planning.
Handwritten parts still contribute declarations through their verified owner and
backlink. Ordinary references to generated model types remain allowed: annotation
ownership is not a replacement for Dart type checking. Do not treat every foreign
tool's `.g.dart` file as Dust-owned output merely because of its suffix.

Production cached commands now use a shared package/source/SDK snapshot. Local
source-URI resolution is implemented against the
[Dart package-configuration rules](https://github.com/dart-lang/language/blob/main/accepted/2.8/language-versioning/package-config-file-v2.md)
and real SDK URI probes. It must reject non-file fetch schemes and invalid path
escapes, retain package/language identities, and resolve dot segments before
reading files. Filesystem location remains distinct from library URI identity:
`package:` and `file:` imports of the same file must not become one declaration.
Compatibility root loading shares local URI translation. The user approved an optional
SDK source-root input on unpublished Rust build/check/watch/route requests.
Normal production loading discovers the active Dart/Flutter SDK; Rust-only tests
can supply synthetic SDK sources without installing Dart. Invalid explicit input
must fail rather than fall back to another SDK. No Dart API or runtime SDK floor
change is proposed.

The request structs, CLI callers and production cached contexts now consume that
optional path; no invalid-input fallback selects a different SDK.
Its discovery follows the first executable `dart` on `PATH`, resolving symlinks
and standalone/Flutter source layouts. Relative `PATH` entries are based on the
process directory, not the requested package root; an opaque proxy requires an installed SDK
bin directory on `PATH` or an explicit Rust request root. Source-map includes and
override order follow the
[SDK specification](https://github.com/dart-lang/sdk/blob/main/pkg/_fe_analyzer_shared/lib/src/util/libraries_specification.dart).
SDK source alternatives remain target-independent: a relevant divergent source
identity is an error, not permission to choose the host's target. Synthetic SDKs
exercise real driver requests without installing Dart in Rust-only tests.

The loader parses reachable export/part interfaces and imports needed by root
metadata or transparent named aliases. Implicit core imports are added only to
graph facts, not handwritten parser facts or raw IR. One source snapshot feeds
discovery, workspace analysis and emission. Per-root fingerprints include actual
source reads, SDK/package metadata, URI identities and diagnostic file IDs. Watch
reloads that graph before comparing dependency fingerprints; route markers use
bound owners rather than text matching. Ownership preflight happens before any
output or successful cache entry can be replaced, including nested derive values.
Root parser failures retain their original diagnostics and notes instead of being
masked by namespace errors through their generated part's backlink. Missing parts
remain source errors; when none resolves to the configured primary output, the
diagnostic also names that output without claiming the missing part is Dust-owned.
Dependency labels retain their own source files. A foreign generated part may
precede the actual Dust part without becoming the selected output directive.

Watch emission remains limited to changed libraries, but its analysis corpus is
the complete current workspace. Unprocessed libraries are lowered against the same
namespace snapshot so an unchanged enum's JSON capabilities cannot disappear from
a consumer-only rebuild. An exact watch/full-build regression verifies those calls
and the subsequent warm-cache hits. This currently re-lowers omitted libraries;
no incremental-analysis performance claim is made.

Real driver regressions cover direct/third-party barrels, named aliases, foreign
shadowing, cold/warm cache hits, independent dependency invalidation, build/check/
watch recovery, source failure, ambiguity labels, stale own output rejection,
foreign generated parts and references to generated model types. A development
smoke build/check using real runtime sources and the latest installed SDK also
passes Dart analysis; it is not the frozen minimum/latest acceptance matrix.
Production regressions additionally cover shared prefixes, qualifier trivia,
conditional import/export ownership, deferred metadata, unrelated missing prefixes,
and direct/nested hidden annotations at all 17 represented metadata sites. Both
primary and short auxiliary Dust ownership headers are recognized; their contents
never supply annotation declarations.

Rust driver fixtures now use valid package maps and explicit synthetic SDKs. CLI
integration fixtures launch the actual binary with a child-only SDK `PATH`; no
global environment change or installed Dart execution is needed. Runtime annotation
declaring sources are embedded from the repository, with controlled test barrels.

The approved plugin contract now declares potential standalone output paths and
producer annotation IDs. The production loader verifies actual handwritten class
ownership before retaining empty placeholders, prunes unsupported claims and
rechecks survivors against the final graph. Existing unmarked output collisions
fail before emission or cache mutation. Focused DB mode receives the same namespace
metadata without enabling route emission. A `.g.dart` suffix alone never grants
ownership, generated contents never supply annotations, and ordinary missing
imports remain errors. Existing route snapshots and cold/check/watch regressions
pass in scoped local runs; complete acceptance remains pending.
The verification and collision rules are recorded in
[producer-owned generated library inputs](generated-library-ownership.md).

The full local Flutter example gate also exposed a separate parser-role boundary:
valid Flutter dependency method bodies using dot shorthands currently trigger
generation-unsupported diagnostics while loading their exported interfaces. Those
diagnostics then poison unrelated annotation lookups. This is tracked in
[#142](https://github.com/dustcodegen/dust/issues/142), implemented separately in
[PR #144](https://github.com/dustcodegen/dust/pull/144) and consumed by the #114 loader.
The [two-view diagnostic contract](namespace-interface-diagnostics.md) retains one
parse while keeping generation and required-interface failures distinct.
Do not solve it by suppressing diagnostics or weakening unsupported-syntax checks
on application roots. The product showcase passes fresh generation/check/analysis;
that is not proof that the Flutter example gate or #114 is complete.

The Rust regression suite parses the real client and server declaring libraries,
then binds both `GET` annotations in either registration order. It verifies
client HTTP normalization and the absence of that normalization for server GET,
plus exact canonical ownership, raw source/span retention, visibility, parts,
aliases, cycles, conditions, ambiguity and labeled input failures. Smaller graph
fixtures use declared absolute identities; their test callback is not a
replacement for production package-config or relative-URI resolution.

Still required under #84 before claiming production namespace support:

1. Finish package/source/SDK URI and platform audits, including full part/output
   path validation. Component tests do not prove every source or platform case.
2. Complete full cold auxiliary-library and parser-role acceptance after the
   approved implementations. Keep Rust-only checks independent of an installed
   Dart SDK, including CLI tests. Full local Flutter lint currently stops at a valid
   instance-field `.forward` initializer in `AnimationController`; #145 requests
   explicit approval for that additional dependency-only boundary. Do not weaken
   the existing initializer guard before approval. The separately tracked valid SQLite generic-call
   regression [#143](https://github.com/dustcodegen/dust/issues/143) still requires
   its own shared parser fix; the namespace projection must not hide it.
3. Complete coverage and sequential local lint/test gates without skipped failures;
   the passing focused ownership regressions are not full acceptance evidence.
4. Qualifier trivia/named-constructor and lexical-shadowing audits, plus graph
   performance/conditional fan-out checks against real dependency graphs.
5. Full end-to-end server generation and minimum/latest SDK evidence using that
   production path. Existing consumer compatibility alone does not prove it.

Only local validation is allowed. All implementation PRs remain unmerged until
separately approved; the only enabled remote workflow is manual Rust release.
