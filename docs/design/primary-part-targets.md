# Resolved primary part targets

Under [#147](https://github.com/dustcodegen/dust/issues/147), production builds
select the parsed `part` whose already-resolved physical target equals the
configured primary output. Matching only the basename is insufficient:
`foreign/model.g.dart` must not stand in for `lib/model.g.dart`.

The immutable namespace snapshot supplies `PrimaryPart::Resolved` with the
original parsed URI field and directive span, or `PrimaryPart::Missing`. The Rust
resolver retains the exact parsed directive and literal source; it does not
parse or normalize URIs again. Relative, escaped, file and package URIs follow
the existing source loader. A foreign handwritten part may precede the real
primary part without becoming Dust-owned output.

A library requiring a primary part fails before emission/cache persistence when
none targets its configured output. Diagnostics label the original part when
present. Partless plugins do not acquire a new part requirement. Build, check,
workspace analysis and watch share this selection contract.

The context-free resolver adapters intentionally retain their existing basename
matching: they have no package/source graph to prove physical target identity.
This is an unpublished Rust contract, not a handwritten Dart API change.
