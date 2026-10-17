# Lexical metadata compiler probes

Audit evidence for [#152](https://github.com/dustcodegen/dust/issues/152), not a
supported Dust feature or generated-code consumer. No packages are required.

`annotations.dart` deliberately defines unrelated local annotations. These do not
stand in for Dust package provenance in the Rust regressions: the resolver test
reads the real Dust HTTP annotation declaration source, and the driver test uses
the existing namespace workspace harness.

`traditional.dart` checks constructor, field, method and parameter metadata.
Reflection asserts identity with the class constant, not just compilation.
`primary.dart` checks a primary class's method/parameter metadata. Both retain a
real imported annotation on the outer class, where the member does not shadow it.

Run with an explicitly selected SDK executable:

```sh
"$DART_MIN" analyze fixtures/namespace_lexical_shadowing/traditional.dart
"$DART_MIN" --enable-asserts fixtures/namespace_lexical_shadowing/traditional.dart
"$DART_LATEST" analyze fixtures/namespace_lexical_shadowing
"$DART_LATEST" --enable-asserts fixtures/namespace_lexical_shadowing/traditional.dart
"$DART_LATEST" --enable-asserts fixtures/namespace_lexical_shadowing/primary.dart
```

Verified locally on Dart 3.6.0 and 3.13.5; the primary probe requires Dart 3.13.
The two associated Rust regressions intentionally remain red pending review of
the [structured use-site scope proposal](../../docs/design/annotation-lexical-scopes.md):

```sh
cargo test -p dust_resolver --test namespace_graph class_constant_metadata_cannot_become_an_imported_client_verb
cargo test -p dust_driver --test driver_tests class_constant_metadata_shadows_an_import_prefix
```

These VM-only reflection probes are not Flutter, server generation, constant
evaluation or complete lexical-scope acceptance evidence. No remote CI is needed.
