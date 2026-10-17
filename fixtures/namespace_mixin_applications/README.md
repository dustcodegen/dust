# Named mixin-application annotation identity

Development regressions for [#155](https://github.com/dustcodegen/dust/issues/155),
not support for generating named mixin-application classes.

`source.dart.snapshot` is an independent compiler/runtime probe. Copy it to
an owned temporary directory as `main.dart`, then run each explicit Dart SDK
with `--enable-asserts main.dart`. Mirrors verifies the actual annotation
instance is the named mixin class, not a guessed imported annotation. This
probe has no package dependencies or network services.

Run `generic.dart.snapshot` the same way in a separate temporary file. It
checks that applied base/mixin arguments and bounded parameters are valid on
both Dart endpoints. A parser regression separately selects source language
versions 3.6 and 3.13; latest-default parsing cannot stand in for that check.

The real-parser Rust tests separately introduce the actual Dust runtime
annotation declarations and catalog owners:

```sh
cargo test -p dust_parser_dart_ts --test extract_tests mixin_applications
cargo test -p dust_resolver --test namespace_graph mixin_applications
```

At the original prototype base `6c30709`, both identity regressions are red:
the parsed class name is empty and local `GET` receives `dust_dart::GET`.
The explicit client/server owner control is green. The implementation must turn
both regressions green and pass the full local gates before acceptance. This
does not enable the server plugin.

## Approved boundary

Retain named-mixin declaration identity, applied base/mixins, generic binders
and original spans structurally. Distinguish its declaration form from ordinary
body classes. Finding its name alone must not imply that Dust knows its
implicit forwarding constructors or can generate a complete derived model.
Until those semantics are supported, reject derivation clearly before output
or successful cache mutation while allowing foreign namespace declarations.

The parser and IR retain a distinct named-mixin class kind and reuse their
structured type parameters, superclass and mixin applications. Relevant
generation requests fail with the original declaration label, including a
local subclass depending on the alias's unsupported construction semantics.
Unowned declarations remain usable for namespace lookup. Driver tests cover
cold/warm build, check, watch, failure preservation and recovery.

No handwritten Dart APIs, versions, dependencies, SDK floor, grammar pin or
workflows change.
Validation is local only; only manual Rust release CI remains enabled.
