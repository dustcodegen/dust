# Listed grammar extraction contracts

This non-publishable, standalone test manifest compiles the real backend source
with the pinned listed grammar instead of the production grammar. It does not
select a parser in the CLI or add a feature to Dust's public API.

Both native grammars export `tree_sitter_dart` and scanner symbols. Linking them
into one test executable can silently resolve both Rust bindings to one grammar.
Keep them in separate executables. Shared contracts also assert their expected
class-node kind, so the wrong grammar cannot pass unnoticed.

Run from the repository root:

```sh
python3 scripts/check_listed_class_parity.py
```

The gate executes the same seven traditional-class contracts with each grammar,
requires every nonempty case, and compares complete `Debug` representations of
the parser-owned facts, including annotation/type values, member ordering and
byte spans. It stores both outputs in `.dart_tool/listed-class-parity/`.
No golden output is rewritten. Negative Python tests reject missing, duplicate,
empty and changed records. Root local Rust test/lint scripts include this gate
and compile/lint the candidate backend separately.

This first slice of [#70](https://github.com/dustcodegen/dust/issues/70) is not
whole-backend adoption. Directives, non-class declarations, enums, queries,
i18n and complete generated-consumer parity still need migration gates before
changing the production dependency. Primary and concise constructors remain
unsupported by Dust. No version bump, release or remote CI is involved.
