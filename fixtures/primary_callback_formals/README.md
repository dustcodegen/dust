# Shorthand callback compiler probes

Evidence for [#92](https://github.com/dustcodegen/dust/issues/92) and the proposed
[source-fact design](../../docs/design/function-formal-types.md), not proof that
Dust currently generates these forms correctly.

Copy each `.dart.snapshot` into a separate owned temporary `.dart` file. Run an
explicit SDK with `--enable-asserts`. No generated parts or placeholder mixins
are used in these language controls.

The eight positive probes compile/execute on Dart 3.13.5: a primary declaring
callback, separate return/function nullability, generic/nested/named callback
parameters, a traditional constructor and preferred explicit function syntax.
The traditional file selects language 3.6; primary files select language 3.13.

`inner_default.dart.snapshot` is intentionally invalid. Dart rejects it with
`DEFAULT_VALUE_IN_FUNCTION_TYPE`; it is not an accepted form to normalize away.

Analyze with a standard rule set and strict casts/inference/raw types. Shorthand
source reports `use_function_type_syntax_for_parameters`; the nested probe has
two such sites. The traditional non-initializing constructor also reports
`prefer_initializing_formals`. These infos are not suppressed or counted as
fatal-info success. The preferred probe must pass fatal-info analysis separately.

The pinned native grammar accepts the inspected primary, nullable-function,
generic and nested signatures. Native AST inspection uses the actual vendored
binding, not a globally selected CLI grammar. Complete Dust semantic/generation
regressions and full acceptance remain pending design approval/implementation.
