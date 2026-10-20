# Listed Dart grammar migration

This source checkout uses the Dart grammar listed in Tree-sitter's
[parser inventory](https://github.com/tree-sitter/tree-sitter/wiki/List-of-parsers):
[UserNobody14/tree-sitter-dart](https://github.com/UserNobody14/tree-sitter-dart).
This is a community upstream, not an official Dart-team parser.

## Active grammar and syntax staging

The active parser uses `vendor/tree-sitter-dart`, pinned at upstream commit
`acfb453395e25cf6a1018e064df3c92e68e38782` with ABI 15 and Tree-sitter 0.25.10.
The separate legacy harness still uses registry `tree-sitter-dart 0.1.0`.
Named concise constructors and dot shorthands are explicitly rejected with
`Dust does not support ... yet`, source labels and a traditional-syntax
workaround. The grammar can parse them; generated-code support is a separate
stage. Primary-constructor work now has separate
[development consumers and limits](primary-constructors.md) under #76.
Updating a local checkout does not change already released binaries.

Continue using traditional in-body constructors for models processed by Dust:

```dart
import 'package:dust_dart/derive.dart';

part 'user.g.dart';

@Derive([ToString()])
class User with _$User {
  const User(this.name);

  final String name;
}
```

No existing handwritten app API, generated output, runtime constraint or
package/CLI version changes in the adoption stage. No released
binary gains primary-constructor support from this work.

## Extraction and adoption checks

The first #70 slice adapts backend-private traditional class/member extraction:
class node names, sibling metadata and bodies, field identifiers/initializers,
factory names/targets and operator names.
Subsequent slices add mixin member/metadata extraction, top-level variable names
and initializer facts, and preservation of generic/named extension-type names.
Twenty-four contracts execute with each grammar in separate native binaries and
compare complete parser facts, including byte spans, with no rewritten snapshots.
Seven cover classes, nested nullable types/defaults, named/super parameters,
modifiers/inheritance, annotated members/parameters, factories and adjacent
method/operator bodies. Eight cover named/unnamed libraries, URI/named parts,
conditional imports/exports, combinators, annotated/enhanced enums, mixins,
extensions/representations, variables, functions and typedefs. Comments and
Unicode source spans must survive too. Four query contracts compare direct db/tx
terminals, generic types, parameters, mapper flags, enclosing names, dynamic SQL
and unchecked-SQL markers. Five i18n contracts compare static keys, fallbacks,
placeholder order, opt-outs, warnings/labels and real shopping-app source parts.
The shared lowering accepts both generic-selector and named-argument layouts;
the i18n query no longer requires grammar-specific constant-constructor fields.
This preserves existing facts; it does not redesign or expand the public parser
surface.

Run `python3 scripts/check_listed_parser_parity.py` locally. The
[isolated harness notes](../../crates/dust_parser_dart_ts/parity/README.md)
explain why both native grammars must not share a test executable. These scoped
contracts and each grammar's full backend unit suite run together. They do not
prove every source form or new-syntax compatibility. Adoption also requires the
full Rust workspace, all package/example suites and freshly generated
minimum/latest SDK consumers to pass with identical generated-file hashes.
The remaining negative probe requires a specific unsupported-generation diagnostic,
not a coincidental parse error or unrelated failure. No test skips or snapshot
rewrites are used to accept a migration regression.

The legacy grammar can miss generic queries with omitted parameters
([#74](https://github.com/dustcodegen/dust/issues/74)). The listed grammar finds
the focused reproducer; that issue retains its separate permanent acceptance
regression. For older binaries, pass an explicit empty list.

## Stages under #68

1. [#69: pinned Rust binding](https://github.com/dustcodegen/dust/issues/69).
   Preserve the upstream C/parser/scanner/header bytes at commit
   `3f0c7a8309d89927e7cc6bef39569c815fddb2e5`, adapt only its outdated Rust
   binding to Dust's existing Tree-sitter runtime, and check provenance,
   loading and exact syntax-tree contracts locally. This preparation initially
   kept the active parser unchanged. See the [snapshot notes](../../vendor/tree-sitter-dart/README.md).
2. [#70: extraction parity](https://github.com/dustcodegen/dust/issues/70).
   Port backend-private extraction of currently supported source into the
   same parser-surface/IR facts. The assessment found 143 focused regressions
   and 26 missing generated files after an unadapted swap; do not change
   expected snapshots or skip tests to hide missing facts. Switch the dependency
   only when existing tests and freshly generated SDK consumers pass.
3. New-syntax generation follows independently. Test induced fields versus
   plain constructor arguments, names, defaults/nullability, annotations,
   generics, `this` bodies and inherited parameters. Grammar-only tests are
   not sufficient. Keep unsupported forms rejected until complete; enum and
   named-concise constructor behavior needs its own scope.

The [SDK gate](sdk-validation.md) must keep proving fresh generation and exact
minimum/latest SDK behavior. Primary syntax gets separate positive latest-SDK
consumers; concise syntax retains its explicit rejection probe. Existing source snapshots and
normal SDK users must keep working throughout the migration.

All validation runs locally. No Actions workflow is added or dispatched;
keep only the existing manual Rust release CI enabled. No version bump or
release is implied. SQLite epic #19 retains its separate platform gates.
