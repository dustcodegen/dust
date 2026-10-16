# Listed grammar extraction contracts

This non-publishable, standalone test manifest compiles the real backend source
with the legacy packaged grammar instead of the active listed grammar. It does not
select a parser in the CLI or add a feature to Dust's public API.

Both native grammars export `tree_sitter_dart` and scanner symbols. Linking them
into one test executable can silently resolve both Rust bindings to one grammar.
Keep them in separate executables. Shared contracts also assert their expected
class-node kind, so the wrong grammar cannot pass unnoticed.

Run from the repository root:

```sh
python3 scripts/check_listed_parser_parity.py
```

The gate executes each grammar's full backend unit suite and the same
twenty-four contracts,
requires every nonempty case, and compares complete `Debug` representations of
the parser-owned facts, including annotation/type values, member ordering and
byte spans. It stores both outputs in `.dart_tool/listed-parser-parity/`.
No golden output is rewritten. Negative Python tests reject missing, duplicate,
empty and changed records. Root local Rust test/lint scripts include this gate
and compile/lint the legacy backend separately. The workspace backend and CLI
use the pinned listed grammar; the harness uses registry `tree-sitter-dart 0.1.0`.

Seven contracts cover traditional classes/members; eight compare complete
library surfaces for directives, enums, mixins, extensions/representations,
variables, functions and typedefs. Assertions require nonempty extraction of
the targeted category before facts are emitted. Four query contracts cover
typed db/tx terminals, mappers, parameters, diagnostic owners, dynamic SQL and
unchecked-SQL markers. Five i18n contracts cover complete translation entries,
fallbacks, placeholders, dynamic opt-outs, warning labels/spans and actual
shopping-app source parts. Samples are parser-fact
fixtures, not standalone app examples; real app analysis remains the SDK gate.

These scoped contracts do not prove every Dart source form. Adoption under
[#70](https://github.com/dustcodegen/dust/issues/70) also requires full workspace
checks and freshly generated minimum/latest SDK consumers; exact generated
hashes must match the pre-adoption baseline. Primary/concise constructors and
dot shorthands remain explicitly rejected until their generated consumers are
implemented. No version bump, release or remote CI is involved.
