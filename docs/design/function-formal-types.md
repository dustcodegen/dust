# Effective types for shorthand function formals

Status: **proposed, pending approval** for
[#92](https://github.com/dustcodegen/dust/issues/92), under primary support #76.
This document changes no Rust contract or handwritten Dart API. It is separate
from generated-binder renaming #103 and inferred primary-field types.

## Reproduced problem

```dart
@Derive([CopyWith(), ToString()])
class const Probe(final bool predicate(String value)) with _$Probe;
```

The callback's effective type is `bool Function(String)`, not `bool`.
The current shared parameter extractors retain only the written return type
before `predicate`; primary field induction copies that same fact. The resolver
then lowers it as a scalar. A successful Dust build can therefore produce
invalid Dart. Preferred `final bool Function(String) predicate` already works.

`ParsedTypeSurface.source` promises exact written type text and a real source
span. Replacing its `bool` text with invented `bool Function(...)` would break
that contract. Merely changing its kind to `Function` would also be wrong:
current function lowering consumes that raw source directly.

The reproduction was refreshed with the clean #156 CLI at `5e73446`: fresh
generation succeeds, but Dart kernel compilation fails because the generated
copy parameter is `bool?` and its replacement is cast to `bool`. No copied
callback part or placeholder mixin rescued the generated result.

## Proposed source facts

Keep `type_source` and `parsed_type` unchanged. Add one optional
`function_formal` fact to constructor parameters, method/function parameters
and primary-induced fields. Ordinary fields/parameters have `None`.
Preserve it through `ResolvedField`; resolved constructors/method parameters
already retain their source surfaces.

Two backend-neutral facts are sufficient, schematically:

```rust,ignore
struct ParsedFunctionFormalTypeSurface {
    name: ParsedLexicalName,
    return_type: Option<ParsedTypeSurface>,
    type_parameters: Vec<ParsedTypeParamSurface>,
    parameters: Vec<ParsedFunctionFormalParameterSurface>,
    nullable: bool,
    span: TextRange,
}

struct ParsedFunctionFormalParameterSurface {
    name: ParsedLexicalName,
    parsed_type: Option<ParsedTypeSurface>,
    function_formal: Option<ParsedFunctionFormalTypeSurface>,
    kind: ParameterKind,
    is_required: bool,
    annotations: Vec<ParsedAnnotation>,
    default_value_source: Option<String>,
    span: TextRange,
}
```

The vector breaks recursive sizing; no boxed wrapper or generic type framework
is needed. New names/bounds/types/metadata use existing source facts and exact
AST spans. `is_required` here describes invocation requiredness, including
required positional arguments. Do not change the existing method parameter's
documented meaning of an explicit `required` modifier.

The signature span covers its actual AST-owned declarator, never a synthetic
`Function` expression. Missing written types remain explicit `None`, not guessed
`dynamic`, `Object?` or an imported same-name type.

## Extraction and scopes

- Use one shared parameter-type extractor for constructor and method/function
  callers. Detect only an owned, direct formal-parameter list after that
  parameter's identifier; never search a default expression or nested callback
  for the outer signature. Preferred function-type syntax keeps its existing
  path and unchanged facts.
- Recursively retain nested callback signatures in their own parameter vectors.
  They must not become extra outer constructor parameters or induced fields.
- Retain callback generic binders, bounds, original name spans and parameter
  groups from the AST. Reuse the existing generic/type extraction helpers.
- Add corresponding callable scope regions for callback binders. Original
  nested parameter metadata must use actual lexical ownership before imports;
  preserve the existing shared preflight/lowering and source-label behavior.
- Plain field/super formals keep their current inference path. Do not change
  ordinary method return-type extraction or class generic binder ownership.

## Effective lowering

Introduce one shared effective-parameter-type lowering operation. Without a
function-formal fact it delegates to existing type lowering unchanged. With a
fact it composes a canonical function type from the retained explicit return,
binders and parameter facts, then uses existing `TypeIr::Function` semantics.
It must not modify raw source, reparse plugin strings or replace substrings.

Parameter groups, names and `required` labels are rendered from facts. Nested
callbacks are rendered recursively. Generic binder names and bound references
stay in their original scopes; this does not approve generated-binder renaming.
Opaque function/record substitution keeps its current fail-closed boundary.

Function nullability and return nullability are independent:

```dart
// Nullable return, non-null callback.
bool? predicate(String value) // bool? Function(String value)

// Non-null return, nullable callback.
bool predicate(String value)? // bool Function(String value)?
```

Do not apply the raw return type's nullable flag to the whole function.
Unsupported inference or malformed effective facts must produce source-located
diagnostics before relevant generation/output/cache mutation. Existing unused
declaration and dependency-interface policies remain unchanged.

The SDK rejects a default inside a function signature, for example
`bool predicate([String value = 'yes'])`. Retain that source/default distinction
and reject it; never silently discard the default to manufacture a valid type.
An outer constructor parameter default is a separate, existing contract.

## Validation and developer experience

The handwritten annotation and copy APIs do not change. Both shorthand and
preferred spelling must have the same callable semantics; preferred spelling
remains the style-clean recommendation.

1. Add exact raw-source/span and effective-fact tests before extraction changes.
   Cover required/optional positional and named parameters, recursive callbacks,
   generic binders/bounds, separate nullabilities, outer defaults, qualifier
   trivia and traditional/primary source. Keep listed/legacy comparisons honest.
2. Assert shared field/constructor/method/function lowering and unchanged ordinary
   return types. No argument/field duplication or unknown-type rescue.
3. Compare exact generated output with preferred-syntax controls. Compile and
   execute generated copy preservation and replacement, not just Dust success.
   Existing unsupported function-field capabilities (such as JSON) must still
   fail correctly instead of seeing an incorrect scalar type.
4. Exercise cold/warm build, check, watch and failed-output/cache preservation.
5. Use two SDK lanes: strict fatal-info analysis for preferred source and its
   generated code; compiler/runtime plus exact expected source-style diagnostics
   for shorthand probes. Do not suppress the SDK's style advice to claim a
   shorthand fatal-info pass. Generated output must remain style-clean.
6. Run current scoped coverage, full local Rust/package/example gates and frozen
   minimum/latest consumers. Minimum traditional output stays identical; primary
   source runs only on an appropriate SDK/language version. Existing #143 is not
   waived by this feature.

The [probe fixtures](../../fixtures/primary_callback_formals/README.md) record
actual compiler behavior, including the invalid-default control. No production
implementation is included in this proposal.

Local probe evidence: all eight valid standalone inputs execute with assertions
on Dart 3.13.5; the traditional input also executes on Dart 3.6.0. Preferred
syntax passes fatal-info analysis under standard lints and strict type checks.
The primary shorthand reports its expected function-syntax style info instead
of passing that strict lane. The invalid inner default gets the SDK's
`DEFAULT_VALUE_IN_FUNCTION_TYPE` error. Native AST inspection with the pinned
vendored grammar accepts the inspected primary, function-nullable, generic and
nested signatures without syntax errors. These facts do not prove Dust callback
generation is fixed.

## Constraints and references

No handwritten wrapper API, version bump, dependency, SDK-floor, grammar-pin,
workflow, release or merge change. Local checks only; keep only manual Rust
release CI enabled. Inferred callback types require their own reviewed policy.

- [Dart primary constructors](https://dart.dev/language/primary-constructors)
- [Accepted primary specification](https://github.com/dart-lang/language/blob/main/accepted/3.13/primary-constructors/feature-specification.md)
- [Prefer generic function type syntax](https://dart.dev/tools/linter-rules/use_function_type_syntax_for_parameters)
