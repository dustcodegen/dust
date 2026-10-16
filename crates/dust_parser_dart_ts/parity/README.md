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
python3 scripts/check_listed_parser_parity.py
```

The gate executes the same fifteen contracts with each grammar,
requires every nonempty case, and compares complete `Debug` representations of
the parser-owned facts, including annotation/type values, member ordering and
byte spans. It stores both outputs in `.dart_tool/listed-parser-parity/`.
No golden output is rewritten. Negative Python tests reject missing, duplicate,
empty and changed records. Root local Rust test/lint scripts include this gate
and compile/lint the candidate backend separately.

Seven contracts cover traditional classes/members; eight compare complete
library surfaces for directives, enums, mixins, extensions/representations,
variables, functions and typedefs. Assertions require nonempty extraction of
the targeted category before facts are emitted. Samples are parser-fact
fixtures, not standalone app examples; real app analysis remains the SDK gate.

These slices of [#70](https://github.com/dustcodegen/dust/issues/70) are not
whole-backend adoption. Further declaration forms, queries, i18n and complete
candidate generated-consumer parity remain gates before changing the production
dependency. Primary and concise constructors remain
unsupported by Dust. No version bump, release or remote CI is involved.
