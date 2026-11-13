# Class Interfaces in IR

Status: proposal. This document scopes adding an `implements` clause to Dust's
parsed surface and IR, and replacing the structural interface detection that
several plugins do today.

For the build pipeline and crate map, see the [Developer Guide](../developer.md).
The server plugin originally motivated this proposal, then avoided the
dependency by requiring `@Extract(Type)` to name the extractor explicitly.

## Problem

`ClassIr` records a superclass, written mixins, and `is_interface`, but nothing
about `implements` (`crates/dust_ir/src/class.rs`). The parser distinguishes an
`implements` clause from `extends` and `with`, but discards the named
interfaces instead of carrying them into the parsed surface.

Plugins that need to know "does this class implement interface X" therefore
guess from method shape. Serde detects `toJson` that way
(`crates/dust_plugin_serde/src/analysis.rs`), and the server design does the
same for `IntoResponse` and `Validatable`. Structural detection has three
costs:

| Cost | Example |
| :--- | :--- |
| False positives | a class with an unrelated `Response intoResponse()` is treated as a response type |
| False negatives | an interface method reached through a mixin or superclass is invisible |
| No diagnostics | Dust cannot say "you declared `implements Extractable` but the method shape is wrong"; the analyzer reports it later, against generated code |

## Non-goals

- Full type resolution. Dust records the written names, not resolved
  declarations, matching how `superclass_name` already works.
- Transitive interface closure. A class implementing `B`, where `B` implements
  `A`, records `B` only. Consumers that need the closure walk it themselves
  using the workspace fact below.
- Generic argument semantics. `Extractable<AuthUser>` is recorded with its
  argument as written; Dust does not check variance or substitution.

## Surface

```rust
pub struct ParsedClassSurface {
    // ...
    /// Written interfaces in source order.
    pub interfaces: Vec<ParsedTypeSurface>,
}
```

`ParsedTypeSurface` already carries the written type shape and span for class
mixins and superclasses, so no second parser type is needed. The remaining
design choice is whether lowered interfaces should reuse `TypeIr` or have a
small `InterfaceApplicationIr`, matching the existing dedicated superclass and
mixin representations.

## Where the change lands

| Crate | Change |
| :--- | :--- |
| `crates/dust_parser_dart` | add `interfaces` to `ParsedClassSurface` (`src/surface.rs`) |
| `crates/dust_parser_dart_ts` | extract them in `src/classes/class_decl.rs`, beside the existing `superclass` field read |
| `crates/dust_ir` | add `interfaces` to `ClassIr` (`src/class.rs`) |
| `crates/dust_driver` | lower them beside the existing superclass and mixin hand-off |
| `crates/dust_plugin_serde` | replace structural `toJson` detection, keeping it as a fallback |

The tree-sitter side still needs one focused proof: existing fixtures show that
`implements` is not mistaken for a superclass or mixin, but do not extract its
types. Confirm the exact child kinds against the vendored grammar before adding
the field.

## Detection policy

Adding the clause would preserve structural detection. Proposed policy:

1. A declared interface is authoritative. `implements IntoResponse` means the
   class is a response type; a wrong method shape is now a Dust diagnostic
   naming the class and the expected signature.
2. Structural detection stays as a fallback, because a class can satisfy a
   contract through a mixin or a superclass without naming it, and because
   removing it would break every project that relies on today's behavior.
3. Where the two disagree, declared but structurally wrong, the declaration
   wins and Dust emits the diagnostic. Silent fallback would hide the typo the
   feature exists to catch.

## What it unlocks

| Consumer | Today | With interfaces |
| :--- | :--- | :--- |
| serde `toJson` | structural, false positives accepted | declared, with a diagnostic |
| server `IntoResponse`, `Validatable` | structural | declared |
| server `Extractable` | irrelevant, the extractor type is named at the call site | bare-type extractor parameters become possible |

The server plugin's possible bare-type extractor form is the largest of these.
It needs one more thing beyond this change: a `dust_server.extractables.v1`
workspace fact mapping value type to extractor type, following the JSON-in-facts
convention the route plugin already uses
(`crates/dust_route_plugin/src/plugin/analysis.rs`), plus an ambiguity
diagnostic when two extractors produce one value type. That belongs to the
server plugin, not here.

## Phases

| Phase | Scope | Exit criteria |
| :--- | :--- | :--- |
| 0 | Confirm the tree-sitter node kinds for `implements`; add parser fixtures covering `implements A`, `implements A<T>, B`, `with M implements A`, and `extends S with M implements A`. | Fixtures assert the extracted names. |
| 1 | `ParsedClassSurface.interfaces`, tree-sitter extraction, `ClassIr.interfaces`, and the lowering hand-off. | A class's interfaces survive to IR, asserted in `dust_driver` lowering tests. |
| 2 | Serde adopts the declared clause with structural fallback and a mismatch diagnostic. | Existing serde fixtures unchanged; new fixture for the mismatch diagnostic. |

Phase 1 remains useful as a structured language fact even if no plugin adopts
it immediately.

## Risks

- **Cache invalidation.** Interfaces become part of the parsed surface, so a
  change to an `implements` clause must invalidate dependent files. Any
  consumer building cross-file facts from interfaces takes a cache dependency,
  the same way the route plugin's facts do.
- **Fallback drift.** Keeping structural detection alongside the declared
  clause means two code paths to test. Every consumer that adopts the clause
  needs fixtures for both paths, not just the happy one.
- **Grammar coupling.** The extraction is tied to tree-sitter-dart node kinds.
  A grammar bump can silently return no interfaces, which would degrade to
  today's behavior rather than fail loudly, so the Phase 0 fixtures are the
  regression net.
