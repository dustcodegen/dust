# Annotation namespace ownership

Status: **approved policy; implementation in progress**. Prerequisite
[#84](https://github.com/dustcodegen/dust/issues/84) for server phase 0
([#82](https://github.com/dustcodegen/dust/issues/82)) under
[#81](https://github.com/dustcodegen/dust/issues/81).

The catalog foundation in [#104](https://github.com/dustcodegen/dust/issues/104)
retains canonical owners and adds unique-only context-free lookup. The two
canonical-retention regressions from [#85](https://github.com/dustcodegen/dust/pull/85)
are covered by permanent passing tests in that implementation. Import-prefix
binding, the graph and the other #84 acceptance gates remain unfinished.
Do not enable the server plugin in the default registry or claim server codegen
works on the strength of this document or the catalog layer alone.

## Approved decision

Implement canonical symbol retention and one library-scoped annotation-binding
policy shared by every resolver/lowering path. Retain unique-name legacy
compatibility only when provenance is unavailable and no known namespace
decision contradicts it. Never use registration order to choose an owner.

For example, these names must remain different:

```dart
import 'package:dust_dart/http.dart' as client;
import 'package:dust_server/annotations.dart' as server;

// @client.GET(...) belongs to dust_dart::GET.
// @server.GET(...) belongs to dust_server::GET.
```

Prefixes are local names, not package identities. A barrel in another package
can re-export a Dust annotation without changing its owner.

No handwritten annotation API, SDK floor, version, dependency or workflow
change is proposed. The internal catalog, directive facts, resolver context
and plugin ownership metadata follow this approved contract. Any further
handwritten API or target-selection design change needs separate approval.

## Original evidence

- `SymbolCatalog` stores one trait/config slot per short name. Registering
  another `GET` overwrites that slot; canonical lookup cannot recover the
  removed owner.
- `resolve_support/annotations.rs` tries canonical lookup and then short-name
  fallback. Dart's written `client.GET` is not the internal canonical ID
  `dust_dart::GET`; a prefix-existence check does not bind it to an owner.
- `annotations.rs::resolve_annotation_ir` is another resolution path.
  Library/top-level lowering must not continue using a different policy.
- `derive_member_names` currently discards prefixes from nested constructor
  values. `@Derive([client.SomeTrait()])` also needs the shared binding policy.
- Imports preserve a prefix and flattened `show`/`hide` vectors. Repeated
  `show` clauses cannot be reconstructed from those vectors. Exports preserve
  only URI/span, and conditional directives preserve only the first URI.
- Existing resolver tests deliberately resolve `@other.Derive` and
  `@other.SerDe` by unique short names without loading the imported package.
  They establish a legacy compatibility contract, not proof that arbitrary
  foreign annotations are Dust annotations in a real Dart library.
- `Validate` can legitimately have both trait and config roles under the
  same canonical owner. That is not an ownership ambiguity.

Code pointers, all under `crates/`: `dust_resolver/src/catalog.rs`,
`dust_resolver/src/annotations.rs`, `dust_resolver/src/resolve_support/`,
`dust_resolver/src/resolve/prefixes.rs`,
`dust_parser_dart/src/surface.rs`, `dust_parser_dart_ts/src/directives.rs`,
and `dust_driver/src/lower/directives.rs`.

## Catalog contract

Retain symbols by canonical ID and semantic role. Keep a separate short-name
candidate index; it must not be a last-writer-wins slot. Deduplicate repeated
routes to the same declaration/owner before considering ambiguity.

Preserve existing registry checks for conflicting plugin claims. A canonical
symbol may retain both supported roles. Role selection follows the existing
context, such as a trait inside `Derive` versus field configuration; binding
an owner must not erase the other role.

Context-free short lookup returns a symbol only when its owner is unique for
the requested role. Multiple owners require an explicit binding or a
diagnostic, not an arbitrary `Option` result interpreted as a foreign marker.

### Implemented catalog layer

Canonical identity/role storage is separate from role-specific short-name
candidate sets. Registration preserves distinct owners and deduplicates repeat
routes to the same owner. Short aliases keep their registered role eligibility;
adding a config role through a different alias does not silently grant that role
to every existing alias. For one unique eligible owner, short lookup retains
trait-first behavior and canonical lookup retains config-first behavior.

The internal registration return value now describes a prior registration of
the **same canonical role**, including through another alias. A different owner
is never a replacement. Existing in-repo registration callers ignore that return
value; handwritten Dart APIs are unchanged.

`SymbolCatalog::candidate_ids` exposes deterministic, unique canonical IDs for
the forthcoming binding/diagnostic layer. For example:

```rust
let mut catalog = dust_resolver::SymbolCatalog::new();
catalog.register_config("GET", "dust_dart::GET");
catalog.register_config("GET", "dust_server::GET");
assert!(catalog.resolve_config("GET").is_none());
assert!(catalog.resolve_qualified_config("dust_dart::GET").is_some());
assert_eq!(catalog.candidate_ids("GET").collect::<Vec<_>>(),
           ["dust_dart::GET", "dust_server::GET"]);
```

`None` from a compatibility lookup is not a diagnostic or proof that an
annotation is foreign. Production binding must still distinguish the outcomes
below and report ambiguity; that integration is not implemented by #104.

## Implemented declaration-origin metadata

[#110](https://github.com/dustcodegen/dust/issues/110) combines the catalog,
[directive facts](directive-namespace-facts.md),
[literal decoder](static-dart-literals.md) and reviewed server runtime contract
on one implementation stack. No GitHub PR has been merged by that integration.

`AnnotationOrigin` explicitly records a canonical symbol, actual package-library
URI and declared class/typedef name. `DustPlugin::annotation_origins` defaults
to an empty slice for existing Rust plugin implementations; empty metadata means
unknown provenance, not a proven foreign declaration. No component is inferred
from the spelling of another, and public barrel URIs are not declaration origins.

All currently claimed built-in annotations have explicit records. Registration
rejects unclaimed/conflicting origins before changing registry state. Exact
repeats deduplicate. Symbols-only registration retains origins only for claims
it actually acquires; it cannot replace a selected plugin's declaration.
Both roles of `Validate` continue to share one actual declaration. The intrinsic
`Derive` grouping origin is available separately, without inventing a plugin
trait/config claim or changing compatibility lookup.

This distinction is essential for the legacy `dust_dart::Query` ID: normal mode
owns the HTTP parameter annotation in `src/http/http_client_annotations.dart`,
while focused DB mode owns the distinct SQL method annotation in
`src/db/annotations.dart`. The effective catalog records the selected declaration
and preserves other HTTP symbols through fallback ownership. It does not pretend
those two Dart classes are the same declaration.

Exact tables in `fixtures/annotation_origins` cover all three driver profiles
(normal, online DB, offline DB). A parser-backed audit verifies every record
against the real runtime class or typedef, including both legacy DB aliases and
the intrinsic group. These are real source/metadata audits, not an imported
namespace or generated-server proof.

The graph must still load actual inputs, resolve URIs/aliases/parts and visibility,
and compare discovered declaration identity with these records. This stage does
not resolve a written prefix, change any handwritten Dart annotation API, enable
server generation or complete #84's binding and cache-preservation acceptance.

## Structured namespace facts

Add ordered visibility combinators for imports and exports, with source spans.
Preserve conditional URI alternatives and their conditions, not only a chosen
string. Preserve original source contracts; do not redefine legacy flattened
fields silently. Parser extraction supplies these facts once.

The binding input must associate registered canonical symbols with their
actual Dart declaration origins. Review explicit ownership metadata for the
existing plugins; do not infer that every name exported by a package belongs
to a similarly spelled plugin. Public entrypoints and application barrels
are resolved through their export graph.

Build the graph from discovered libraries and package-config-resolved sources.
Do not fetch imports from the network, scan all dependency caches, or introduce
plugin-local source parsers. A failed required read is a diagnostic, not a
license to select a short-name fallback.

Each library's namespace must account for:

- Prefixed and unprefixed imports, local declarations and supported aliases.
- Multiple imports contributing to the same prefix.
- Ordered combinators: successive `show` clauses intersect the visible set.
- Re-export visibility, diamonds and cycles; identical origins remain one
  binding, but distinct same-name declarations do not.
- Parts using the namespace of their owning library rather than an empty
  per-file import list.

Memoize export results and use a terminating cycle-aware computation. Record
the actual input/dependency set so changing a barrel or visibility clause
invalidates affected discovery, analysis facts and generated caches.

## Binding outcomes and legacy policy

The shared operation must distinguish these outcomes, rather than collapsing
all of them into `None`:

| Outcome | Required behavior |
| :--- | :--- |
| One visible registered declaration | Bind its canonical owner, then select the role appropriate to the annotation site. |
| A proven foreign declaration | Leave it foreign; never normalize it as a Dust client/server marker. |
| Hidden name or undeclared prefix | Do not use fallback to bypass the visibility error. |
| Distinct visible declarations/owners | Report ambiguity at the annotation, with useful origin/import labels. |
| Provenance unavailable in a legacy/context-free call | Retain short-name compatibility only when one canonical owner is eligible. |
| Missing required input or unresolved ownership with overlapping candidates | Diagnose; do not choose an owner or write output/cache. |

An unresolved alias is not proof of a foreign declaration. Re-exports and
transparent aliases whose origin is known bind normally. Opaque aliases may
use the unique-only legacy rule when provenance genuinely remains unknown;
they must never select between overlapping client/server names.

Keep the old context-free resolver entrypoint as a compatibility adapter if
needed. The driver must supply a real library binding context. Existing
unique-alias tests stay valid in that adapter, while new production-context
tests prove that a known foreign class is not hijacked.

This is an approved intentional tightening relative to blindly treating every
foreign same-name declaration as Dust, not an accidental consequence of
replacing the map. It still requires the production binding implementation.

## Conditions and deferred imports

Code generation must not silently choose a platform-specific annotation owner
from the host machine. If all relevant conditional branches lead to the same
canonical declaration, ownership is target-independent. Otherwise produce a
clear diagnostic until a reviewed target-selection contract exists. Unrelated
conditional imports must not trigger blanket rejection.

Do not count such a diagnostic as support for target-dependent annotation
ownership. Adding target inputs or CLI switches would need separate approval.

Dart rejects metadata requiring a deferred library constant before it is
loaded. Do not turn a deferred annotation into a normal Dust annotation through
short-name fallback.

## One policy across all sites

Use the same binding result for library directives, classes/enums and variants,
fields, constructors, methods, parameters, top-level declarations and nested
`Derive` constructor values. Preserve raw annotation names, arguments, source
spans and both trait/config capabilities.

Bind before feature normalization. In particular, a server `GET` must never
reach the Dio-client normalizer, even if both plugins register the same short
name. Discovery and partless top-level handlers need the same ownership facts.

## Implementation acceptance

1. Canonical catalog tests retain both `GET` owners and both `Validate` roles
   in either registration order; all #85 regressions pass.
2. Exact parser facts cover ordered import/export combinators, conditional
   alternatives, prefixes, parts and original spans.
3. Resolver tests cover direct/re-exported/prefixed/unprefixed names, shared
   prefixes, duplicate-origin diamonds, cycles, foreign declarations, legacy
   aliases and ambiguity at every annotation site.
4. Driver regressions prove discovery, dependency/cache invalidation and
   unchanged prior output/cache on ownership errors. Existing client/data/
   Flutter consumers retain identical generated bytes.
5. Full sequential local lint/test gates, accurately scoped changed-code
   coverage and fresh minimum/latest SDK generation/analysis/execution pass.
   Only then enable the server registry in its reviewed implementation stage.

No automatic merge, version bump, release or remote CI dispatch. Only the
manual Rust release workflow remains enabled.

## Language experiments and limits

Standalone Dart-VM experiments used stand-in annotation classes, not Dust's
runtime annotations or generated output:

- Dart 3.6.0 and 3.13.5 each passed fatal-info analysis with recommended lints,
  strict casts/inference/raw types and 12 tests.
- They covered prefixes, a shared prefix, filtered/mixed/diamond/cyclic
  re-exports, foreign declarations versus foreign barrels, conditional
  branches, parts and metadata identity at library/member/parameter sites.
- Five negative compiler probes per SDK rejected hidden export names,
  ambiguity with and without a prefix, and deferred-library metadata.
- Repeated-`show` semantics were checked separately: `GET` remained available,
  while `POST` was rejected. Dart 3.13.5 warns about `multiple_combinators`;
  this is a language-acceptance probe, not a strict-analysis pass for that
  unusual source form. No warning was suppressed.

The reflection tests use `dart:mirrors` only in the temporary VM experiment.
They do not add a dependency to Dust or establish Flutter/web behavior.
Those experiments did not change production code or permanent regressions.
The catalog implementation now adds real Rust regressions; its local suite,
coverage and SDK evidence are recorded in its implementation PR. They do not
establish the remaining graph/all-site binding, failure preservation or full
server codegen acceptance.
