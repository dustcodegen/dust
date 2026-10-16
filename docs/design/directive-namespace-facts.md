# Directive namespace facts

Status: **implemented facts layer; binding unfinished** under
[#106](https://github.com/dustcodegen/dust/issues/106), following the
[approved namespace policy](https://github.com/dustcodegen/dust/pull/97)
for [#84](https://github.com/dustcodegen/dust/issues/84).

This layer is based on the listed-grammar/primary stack in draft
[#98](https://github.com/dustcodegen/dust/pull/98). Catalog retention
[#105](https://github.com/dustcodegen/dust/pull/105) is a separate layer;
the graph implementation must integrate both. Do not enable the default
server registry or count this as completed server codegen.

## Contract

The shared parser retains source facts, not a selected platform namespace:

```dart
import 'base.dart' if (dart.library.io) 'io.dart'
    show GET, POST show GET hide POST;
```

`ParsedNamespaceDirective` and `NamespaceDirectiveIr` retain the default literal,
every conditional alternative/parenthesized condition, and all three visibility
clauses in order. A future binding computation must apply successive shows as
intersections, not flatten them into a union. No condition is evaluated here.

URI facts contain **exact literal source**, including quotes, raw prefixes and
escape spelling. They are not decoded filesystem paths. Combinators retain
their operation, written names and clause spans. Conditional facts retain both
the full `if (...) 'uri'` span and separate condition/literal spans. Comments
inside a condition or clause are retained by those original UTF-8 byte ranges.

Parts and URI-form `part of` carry the same exact literal facts. Named `part of`
retains its existing library-name representation and has no URI literal. IR
lowering attaches the owning `FileId` without rewriting source or byte offsets.

The legacy `uri`, flattened import `show`/`hide`, prefix and deferred fields are
unchanged. They remain compatibility data, not sufficient evidence for ordered
visibility or conditional ownership. Namespace/literal facts are optional for
synthetic callers that did not supply source facts; lowering leaves them absent
rather than inventing quoted text or spans. Missing facts are never proof that
an annotation is foreign, and parser diagnostics must still stop invalid input.

## Verification

Permanent exact parser contracts cover multiple/interleaved shows and hides,
multiple conditions, comments, shared/deferred prefixes, raw/escaped literal
spelling, parts and UTF-8 spans. Whole-fact listed/legacy grammar comparison
includes the same fixture. Driver tests carry real parsed facts through
resolution and IR for traditional and primary class headers, and verify that
synthetic unsupplied facts are not reconstructed.

The fixture is a parser/lowering contract, not an application with resolvable
barrels or proof of import visibility semantics. Full local gates and fresh
minimum/latest SDK consumers remain required; no generation or handwritten API
change is intended by adding these facts.

## Remaining work

- Verify URI semantic decoding before reading graph inputs; the shared static
  literal decoder's Unicode-escape mismatch is tracked separately in
  [#107](https://github.com/dustcodegen/dust/issues/107).
- Associate canonical symbols with actual declaration origins, construct the
  import/export/part graph, and use one binding policy at every annotation site.
- Validate conditional equivalence, foreign/hidden/ambiguous/missing outcomes,
  discovery/dependency invalidation and affected output/cache preservation.
- Complete server phase 0 and the server generator's reviewed runtime contract.

No version/dependency/SDK-floor/workflow change, automatic merge, release or
remote CI dispatch. Only manual Rust release CI stays active.
