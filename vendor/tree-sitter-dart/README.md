# Pinned listed-upstream Dart grammar

Active source-checkout binding for migration
[#68](https://github.com/dustcodegen/dust/issues/68), prepared in
[#69](https://github.com/dustcodegen/dust/issues/69) and adopted under
[#70](https://github.com/dustcodegen/dust/issues/70). The standalone parity
harness retains the packaged grammar in a separate binary. Accepting primary
constructors here did not by itself make them supported by Dust's generators;
the separate extraction and generation work remains independently tested. This
source change does not update already released binaries.

## Origin and changes

Source: <https://github.com/UserNobody14/tree-sitter-dart>, exact commit
`acfb453395e25cf6a1018e064df3c92e68e38782`. This is the community grammar listed
by Tree-sitter, not a Dart-team-maintained parser.

`grammar.js`, `src/` and `LICENSE` are byte-identical upstream copies. No C
grammar changes, local parser regeneration, scanner patches or header changes
were made. The updated grammar and generated parser include upstream's generic
constructor-receiver and record-type disambiguation fixes. `UPSTREAM.json`
records their SHA-256 values and original metadata:

Local Git attributes disable line-ending conversion for those pinned files.
A regression checks their bytes after a `core.autocrlf=true` checkout into an
owned temporary directory; provenance tests therefore also require local Git.

| Local path | Original upstream path |
| --- | --- |
| `upstream/Cargo.toml` | `Cargo.toml` |
| `upstream/package.json` | `package.json` |
| `upstream/build.rs` | `bindings/rust/build.rs` |
| `upstream/lib.rs` | `bindings/rust/lib.rs` |

The active local Cargo manifest and Rust binding adapt the upstream's old
Tree-sitter 0.22 interface to `tree-sitter-language::LanguageFn`. Tests use
exactly Tree-sitter 0.25.10, the runtime Dust already uses. The crate retains
upstream's `0.0.1` metadata version, is non-publishable, and is excluded from
the root workspace, but is now a production path dependency.

The local build script preserves upstream C compiler settings and compiles
the checked-in C sources. Its only refactoring is sharing the two source-file
steps and watching the header directory. It does not fetch anything or invoke
Node, npm, Tree-sitter CLI or a Dart SDK. A C compiler is still needed when
building Rust source, as with the existing grammar dependency.

The unsafe foreign-function binding is confined here, as in ordinary grammar
crates. The Dust parser backend's `forbid(unsafe_code)` remains unchanged.
No new application API, runtime option or parser selection flag is added.

## Licensing and native warning

The original LICENSE is preserved unchanged, including copyright and permission
notices. Upstream's Rust Cargo metadata declares MIT and its LICENSE contains
the MIT permission text. The unrelated npm package metadata says ISC; that
original file is retained for audit, not used to install or license a Node
dependency. This snapshot does not alter upstream's grant or claim Dart-team
ownership. License/provenance changes require review when updating the snapshot.

Upstream's C scanner contains an unused static `skip` helper. Clang can report
that existing warning; the source is intentionally unchanged. No additional
warning suppression is introduced. Rust clippy remains fatal for warnings.

## Local checks

```sh
python3 scripts/check_listed_dart_grammar.py
python3 -m unittest discover -s scripts -p 'test_listed_dart_grammar.py'
cargo fmt --manifest-path vendor/tree-sitter-dart/Cargo.toml -- --check
cargo clippy --manifest-path vendor/tree-sitter-dart/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path vendor/tree-sitter-dart/Cargo.toml --locked
```

Root Rust formatting/lint/test scripts run these checks automatically. Source
hash checks are offline; they detect changes against the recorded snapshot,
not an independently authenticated remote revision. Reviewers must compare
snapshot updates with the exact upstream commit. First-time Rust dependency
resolution needs the ordinary Cargo registry cache or network; the grammar
itself is already present and needs no Git access or generation tools.

Tests assert exact AST contracts for traditional, primary and named-concise
constructors, exercise generic/named/body/enum/inheritance grammar forms and
generic constructor receivers, and reject malformed source. Dust extraction and
generated behavior remain separate compatibility gates.
