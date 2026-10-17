# Lexical annotation ownership

Status: **proposal, not approved or implemented**. Tracks
[#152](https://github.com/dustcodegen/dust/issues/152) under production namespace
loading #114, namespace ownership #84 and server epic #81. This is independent
of the upstream grammar submission for #143; the grammar pin stays unchanged.

## Reproduced boundary

`NamespaceScope::bind(name, catalog)` searches a library namespace. It has no
metadata-use position or enclosing lexical scope. This is insufficient even
without inheritance, type inference or primary constructors:

```dart
import 'package:dust_dart/http.dart';

class Api {
  static const GET = Object();

  @GET
  void call() {}
}
```

Dart attaches the foreign constant to `call`. Dust currently assigns
`dust_dart::GET`. A constant named `d` also shadows an import prefix `d` at class
member metadata, but Dust reports that `d` is an import prefix rather than an
annotation. Outer metadata on the class still sees the import, not that member.
The standalone [compiler probes](../../fixtures/namespace_lexical_shadowing/README.md)
verify actual metadata instances, including a primary class on Dart 3.13.

## Proposed contract for review

1. Add parser-owned lexical scope/declaration facts with exact names and spans,
   parent relationships and explicit visibility regions. Include relevant static
   members even when they are not generated instance fields. Retain separate
   existing generation surfaces and all exact-source contracts. Do not infer
   declarations from plugin source-string scans or annotation short names.
2. Add a use-site namespace operation taking the source file and metadata span.
   Search the innermost applicable scope first, then its parents and imports.
   Resolve the first component of a qualified name the same way; spelling that
   matches an import prefix is not sufficient. Preserve the existing library-only
   and context-free adapters for their documented callers, but route production
   metadata through the use-site operation.
3. Treat lexical declaration identity and constant-value provenance as separate
   decisions. A local declaration shadows an import even when its value is
   unknown. Do not rebind it to the imported annotation or label it foreign merely
   because a local declaration exists.
4. Retain minimal structured constant-initializer facts where needed to prove a
   direct constructor/constant reference. Reuse parser-owned value/type facts
   where suitable. A constructor invocation must bind its actual declared owner;
   source text such as `Object()` is not an ownership proof by itself. Track
   references by declaration and terminate alias cycles. No general constant
   evaluation, inferred type checking or generated-file multi-pass is proposed.
5. A proven foreign constant remains unowned. An opaque lexical constant that
   could affect a registered annotation must fail with use/declaration labels,
   rather than using short-name fallback. Resolving a constant to an actual Dust
   annotation must not silently drop its constructor arguments: either carry
   supported structured values into the existing normalizer or reject that form
   clearly. Review this boundary before implementation; existing unique-owner
   legacy behavior for genuinely unavailable provenance stays unchanged.
6. Share the operation across discovery, ownership preflight, producer-output
   verification, resolver normalization and every metadata-lowering path. Retain
   lexical initializer dependencies in the existing read-set/cache/watch model.
   An ownership failure must preserve successful output/cache state.

These change unpublished Rust parser/resolver contracts, not handwritten Dart
annotations. Exact fact layouts and scope visibility need tests before emission
changes. No server registry enablement is authorized by this proposal.

## Required acceptance

- The two red regressions become green without suppressions, skipped tests,
  opaque source rewriting or weakening root diagnostics.
- Class outer metadata versus body metadata; constructor, field, method and
  parameter sites; traditional and primary headers; explicit prefixed real Dust
  annotations; nested derive values and unrelated-library controls retain their
  actual scope and owner. Cover any additional represented declaration scopes
  before claiming all-site production support.
- Qualified constant references, known foreign constructors, supported Dust
  constants, unknown initializer shapes and alias cycles never fall back across
  a known shadow. Missing required sources remain errors.
- Driver cold/warm build, check and watch agree. Failed ownership leaves previous
  generated output and successful cache bytes unchanged. Unrelated libraries
  keep their ordinary owners and outputs.
- Exact parser/IR/output contracts, scoped changed-line coverage, minimum/latest
  Dart evidence and full local gates pass. The independent #143 full-Flutter
  failure is not waived by this work.

No versions, dependencies, SDK floors, grammar pin or workflow changes. Validation
is local only; keep only manual Rust release CI enabled. No merge or release.
