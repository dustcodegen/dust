# Static Dart literal values

Implemented prerequisite for [#107](https://github.com/dustcodegen/dust/issues/107)
and namespace binding [#84](https://github.com/dustcodegen/dust/issues/84).
This is a shared value-decoding fix, not a completed namespace graph or server
generator.

`dust_dart_syntax::parse_static_dart_string_literal` decodes one literal to
`Option<String>`. It supports single/double quotes, lowercase `r` raw strings,
triple quotes, control/identity escapes, `\xHH`, `\uHHHH` and `\u{H...}` (one
through six hexadecimal digits, at most `10ffff`). Valid surrogate pairs become
one Unicode scalar. Physical multiline line endings normalize to LF, and a
whitespace-only first physical line is omitted, including backslash-prefixed
whitespace in raw strings. Escaped physical CR is an identity escape: it stays
CR rather than undergoing normal physical-line-break normalization.

These rules follow the [Dart language specification, Strings section](https://spec.dart.dev/DartLangSpecDraft.pdf)
and real minimum/latest compiler comparisons. Dart can represent unpaired
UTF-16 code units, but Rust `String` cannot: the helper explicitly returns
`None` for those values, never replacement characters. It also rejects malformed
escapes, uppercase `R`, unescaped single-line line breaks, interpolation and
adjacent-literal concatenation. The last two may be valid Dart expressions;
they are deliberately outside this **single static literal** contract.

```dart
import '\u0061.dart'; // Same URI string as 'a.dart'.
queryScalar<int>('\u0053ELECT \x31', []).fetchOne(db); // Checked as SELECT 1.
```

Consumers use semantic values without overwriting source. Parser annotation
arguments retain exact spelling and byte spans. Unsupported values remain
expressions, not strings containing the source text. IR string accessors respect
structured non-string facts; raw-only compatibility calls use this same static
decoder. Checked direct and positional DAO SQL paths therefore validate
the same SQL online and offline. The i18n scanner decodes keys, defaults and
placeholder names through the same helper.

The public Dart SQL annotation is `@Query(sqlLiteral)` (positional). Named `sql`
fixtures exercise low-level raw/structured IR compatibility; they do not prove
a compilable `@Query(sql: ...)` Dart API or add that constructor form.

The separately documented `parse_string_literal` helper remains a shallow
delimiter-stripper, unchanged. It is not suitable for SQL or URI identity.
Directive facts still retain exact literals and spans; graph consumers must
decode those through the verified helper and fail on unsupported values rather
than falling back to legacy URI text or treating them as foreign annotations.

## Regression evidence

`fixtures/static_strings/values.txt` contains supported exact literal sources;
`utf16_only.txt` contains valid Dart values the Rust API explicitly rejects.
Records are separated by a line containing `---`. Each header has a case name
and tab-separated expected hexadecimal UTF-16 units, followed by exact source.
A single expectation applies to all line endings; three expectation columns
describe LF, CRLF and CR where escaped physical line breaks differ.

Rust reads this corpus directly. `scripts/sdk_static_strings.py`, called by the
[local SDK gate](../usage/sdk-validation.md), stages `package:test` assertions
from the same corpus on minimum/latest Dart, with all three physical line
endings. These deliberately unusual literal payloads are compiler/runtime
probes, not ordinary analyzer-style examples. The probe also imports a real
`a.dart` through Unicode- and hex-escaped URIs. Existing analyzer gates remain
unchanged; owned probe tests are removed even after failure. Rust regression
tests additionally cover malformed input, static boundaries, exact source/spans,
IR fallback rejection, i18n and SQLx-backed online/offline direct/DAO validation.

Corrected multiline SQL values can differ from prior Dust cache keys because
the first physical whitespace-only line no longer becomes an extra newline.
Rebuild checked SQL metadata online before relying on offline mode. Native
database contents and handwritten SQL source are not rewritten.

No dependency/version/SDK-floor/workflow change, merge or release is included.
Run CI only locally; only manual Rust release CI remains active.
