# Namespace interface diagnostics

Status: user-approved design for [#142](https://github.com/dustcodegen/dust/issues/142),
under production namespace loading [#114](https://github.com/dustcodegen/dust/issues/114).
The parser contract is being implemented separately from production driver wiring.
This does not enable dot-shorthand generation or change the public Dart API.

## Observed problem

The current parser puts syntax errors, language-version failures and unsupported
generation forms into one `ParseResult.diagnostics` vector. The loader uses that
same vector to decide whether an imported/exported interface is usable.

Real Flutter sources in Dart 3.13.5 use dot shorthands inside method bodies.
Tree-sitter recognizes those expressions, but Dust deliberately rejects them for
generation. Exporting such a library therefore poisons unrelated annotation
ownership, including `override`, although its declaration does not depend on the
unsupported method body. The local Flutter benchmark gate reproduces this in
`text_painter.dart` and `animation_controller.dart`.

## Proposed contract: two views of one parse

Keep the existing complete `ParseResult.diagnostics` behavior for generation and
`has_errors()`. Add a separately named namespace-interface diagnostic projection
to the unpublished Rust parse result. The backend produces both from one AST and
the same effective language version, with unchanged source facts and spans.

Classify diagnostics structurally during extraction. Do not recognize categories
by message or label text. The namespace projection must retain:

- Every actual syntax error or missing token, including inside executable bodies.
- Every language-version error, even in bodies not used for ownership.
- Unsupported generation forms in declarations, metadata, parameter defaults,
  field/top-level initializers and aliases until their interface facts are proven.
- Unsupported forms whose location or effect on interface extraction is unknown.

Only a grammar-recognized generation limitation proven to be inside a complete
executable body may be excluded from this projection. Initially, this exception
is limited to dot-shorthand expressions within AST-owned function/method bodies.
It must not excuse concise constructor signatures, annotations, unsupported
initializers, or broken syntax nested under an otherwise unsupported node.

The diagnostic walker must still traverse for syntax errors and language-feature
occurrences. An early return after spotting a generation limitation cannot hide
those failures. Root diagnostic ordering, notes and spans must stay unchanged
where the current source is correctly diagnosed.

## Driver integration

Store both projections on the immutable source unit. Graph edges use the namespace
projection; selected generation roots use the complete diagnostics. This includes
an imported workspace file that is also a generation root: its interface may be
read, but its own unsupported generation still fails before output replacement.

There is no parse-twice discovery pass, source reread, host-condition selection,
message filtering, generated-source annotation provenance, or foreign fallback
for required malformed inputs. `ParseOptions` and `SourceKind` need not gain a
mode: a source can have both roles within the same build.

Reach: parser result constructors and fake backends; tree-sitter diagnostic
extraction; source-unit storage and graph failure checks; parser/driver/CLI tests.
The existing compatibility adapter and frozen parser parity need explicit checks.

## Validation before acceptance

1. Compare exact parser facts for the two views; original source spans survive.
2. A required annotation exported beside a valid dot-shorthand method body binds
   and generates the same bytes as the equivalent traditional-body fixture.
3. An annotated root with that body still reports the original unsupported-form
   error and retains prior output/cache bytes in build, check and watch.
4. Syntax and language-version errors inside dependency bodies remain fatal when
   that interface is required. Nested failures cannot disappear behind a parent
   unsupported-form node.
5. Unsupported metadata, aliases, initializers and constructor signatures remain
   errors; foreign annotations cannot reach a registered same-name owner.
6. Run scoped coverage, all parser/driver regressions, strict Rust lint, complete
   sequential local lint/test gates, and fresh minimum/latest Dart/Flutter consumers.

The independent cold generated-library/producer contract is approved separately;
its implementation remains under #114. Neither this design nor passing parser
interface tests proves production integration or server codegen complete.
