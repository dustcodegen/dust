# Listed Dart grammar migration

Dust is preparing the Dart grammar listed in Tree-sitter's
[parser inventory](https://github.com/tree-sitter/tree-sitter/wiki/List-of-parsers):
[UserNobody14/tree-sitter-dart](https://github.com/UserNobody14/tree-sitter-dart).
This is a community upstream, not an official Dart-team parser.

## Current behavior stays unchanged

The active parser still uses the existing `tree-sitter-dart 0.1.0` package.
The prepared binding in `vendor/tree-sitter-dart` is not yet a production
dependency. Class-header primary constructors and named concise `new`
constructors still fail in Dust with the documented diagnostic. Updating an
SDK or finding a grammar that parses them is not Dust generation support.

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
package/CLI version changes in the binding preparation stage. No released
binary gains primary-constructor support from this work.

## Extraction parity in progress

The first #70 slice adapts backend-private traditional class/member extraction:
class node names, sibling metadata and bodies, field identifiers/initializers,
factory names/targets and operator names. The production grammar stays unchanged.
The same seven contracts execute with each grammar in separate native binaries
and compare complete parser facts, including byte spans, with no rewritten
snapshots. They cover nested nullable types/defaults, named/super parameters,
modifiers/inheritance, annotated members/parameters, factories and adjacent
method/operator bodies.

Run `python3 scripts/check_listed_class_parity.py` locally. The
[isolated harness notes](../../crates/dust_parser_dart_ts/parity/README.md)
explain why both native grammars must not share a test executable. These scoped
contracts do not prove whole-backend or new-syntax compatibility; #70 and #68
stay open until all remaining extraction and generated-consumer gates pass.

## Stages under #68

1. [#69: pinned Rust binding](https://github.com/dustcodegen/dust/issues/69).
   Preserve the upstream C/parser/scanner/header bytes at commit
   `3f0c7a8309d89927e7cc6bef39569c815fddb2e5`, adapt only its outdated Rust
   binding to Dust's existing Tree-sitter runtime, and check provenance,
   loading and exact syntax-tree contracts locally. Keep the active parser
   unchanged. See the [snapshot notes](../../vendor/tree-sitter-dart/README.md).
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
minimum/latest SDK behavior. Its negative probes remain unchanged until the
corresponding new syntax has full Dust support. Existing source snapshots and
normal SDK users must keep working throughout the migration.

All validation runs locally. No Actions workflow is added or dispatched;
keep only the existing manual Rust release CI enabled. No version bump or
release is implied. SQLite epic #19 retains its separate platform gates.
