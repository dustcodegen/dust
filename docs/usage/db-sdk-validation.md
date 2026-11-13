# Local SQLite SDK and native consumer validation

This is the independent consumer gate for [#46](https://github.com/dustcodegen/dust/issues/46)
under SQLite epic [#19](https://github.com/dustcodegen/dust/issues/19).
It does not publish packages, change selected SDKs or invoke remote CI.

## Run

Use an explicit Dart executable, not an assumed `stable` alias:

```sh
python3 scripts/sqlite_sdk_consumer.py --dart /path/to/dart-sdk/bin/dart
```

To include the existing generated Flutter shopping consumer, name an installed
Puro environment without selecting it as the project/global default:

```sh
python3 scripts/sqlite_sdk_consumer.py \
  --dart /path/to/flutter/bin/cache/dart-sdk/bin/dart \
  --flutter-env your-installed-env
```

The legacy `bash scripts/test_sqlite_min_sdk.sh` entrypoint delegates to the
same gate using the Dart executable on PATH. Verify the reported version.
Requirements are Python 3.10+, native Dart, build-hook prerequisites/network
access, and Puro only for the optional Flutter run. The recorded SDK matrix is
macOS arm64; the separate [Linux arm64 gate](db-linux-validation.md) reuses the
standalone consumer. Neither is Windows or mobile build certification.

Each invocation creates a fresh owned temporary staging root and retains its
path, numbered command logs, resolved dependencies/locks and `proof.json` for
review. No existing project lock or `.dart_tool` directory is copied. Consumer
source/manifests are copied unchanged, while explicit path overrides point to
the actual Dust source packages. Staging has no Pub workspace. Repository
relative paths remain valid through staging-only package links.

Only lint tooling changes by compiler: very_good_analysis 10.1.0 for Dart
3.10/3.11; the existing 10.3.0 for 3.12+. The latter requires Dart 3.12 and
contains rules older analyzers do not recognize. Production constraints are
never relaxed. Pub resolves SDK-compatible transitive packages independently;
this tests allowed consumer resolution, not the root lock on every SDK.
If Flutter migrates the staged analysis options, the runner restores the
original options before no-Pub analysis/tests. No source warnings are suppressed.

The current fixture and staged Flutter consumer pin sqlite3 3.7.1, which bundles
SQLite 3.54.0. Standalone Dart 3.10.0 remains supported; Flutter SQLite consumers
require Flutter 3.47.0 or newer. The dated host results below record tested
versions. Current-bundle Android/iOS execution remains unverified.

## What must pass

- Strict analysis, unchanged formatting and all seven standalone tests.
- Sync and persistent-worker direct db/tx calls; typed/BLOB/nullable decoding,
  caller-isolate mapping, exact committed rows, migration history/checksum and
  applied timestamp preservation, integrity after close/reopen.
- JIT execution, `dart build cli`, then execution of the **whole relocated
  bundle** after the staged source is moved away. Compare native version/source
  and migration facts, not timestamps from different test databases.
- Record SHA-256 of the bundle's native library assets and the actual loaded
  `sqlite_version()` / `sqlite_source_id()`, not merely the binding version.
- For the optional Flutter run: strict analysis and the complete shopping suite,
  including generated synchronous/background facades and checked query bridges.

The runner ignores only Dart 3.10's exact leading `Running build hooks...`
stdout progress when parsing JIT JSON; unexpected output still fails. Raw logs
are retained. CLI assertions are ordinary runtime checks, so AOT cannot silently
disable them. Python regression tests cover staging and proof validation:

```sh
python3 -m unittest discover -s scripts -p test_sqlite_sdk_consumer.py -v
```

## SDK boundaries

The package's declared floor remains **stable Dart >=3.10.0**, unchanged.
The minimum Flutter version for SQLite consumers is **3.47.0**. This is separate
from the core `dust_flutter` minimum of 3.29.0. The SQLite runtime stays Dart-only;
Flutter constraints belong to Flutter applications such as the shopping example.

The older matrix below used Flutter 3.38.1 with stable Dart 3.10.0. Flutter
3.38.0 bundled Dart `3.10.0-290.4.beta` and failed the stable Dart constraint
before execution; it was not a passing floor test. See Flutter's
[upstream SDK packaging issue](https://github.com/flutter/flutter/issues/178400).
A separately downloaded stable Dart
3.10.0 archive was checked against its published SHA-256 before use.

The historical matrix also used Flutter 3.44.1 / Dart 3.12.1 and Flutter
3.47.7 / Dart 3.13.5 as separate targets. Exact commands and native proof, not a
nearby formatter patch, establish execution. The earlier shopping analyzer's
nullable-boolean diagnostics are tracked separately in
[#47](https://github.com/dustcodegen/dust/issues/47).

## Patched bundle evidence (2026-11-12)

The sqlite3 3.7.1 configuration passed these macOS arm64 checks, including the
exact Flutter 3.47.0 minimum and the separately measured 3.47.9 patch.

| SDK | Standalone format / analysis / 7 tests / JIT / relocated AOT | Shopping analysis / 85 tests |
| --- | --- | --- |
| Stable Dart 3.10.0 | Passed | Not a Flutter SDK |
| Stable Dart 3.14.0 | Passed | Not a Flutter SDK |
| Flutter 3.47.0 / Dart 3.13.0 | Passed | Passed |
| Flutter 3.47.9 / Dart 3.13.7 | Passed | Passed |

The Flutter 3.38.1 attempt failed Pub dependency resolution; its shopping suite
did not execute. The standalone Dart 3.10.0 pass does not establish Flutter
3.38.1 compatibility. Current-bundle Android/iOS lifecycle and packaging proof
remains separate and unverified.

All recorded bundled JIT/AOT engines reported SQLite 3.54.0, source ID
`2026-10-09 15:46:58 be8d059e9a49089ab2dce5ed26dd87aaf598fdc5fbe0b107c0dd758464bcd5e3`.
The downloaded macOS arm64 asset matched sqlite3 3.7.1's published SHA-256:
`d91d35fe3a3db679ce44d5e9d95bacacb11a842c94e7cdcefd6e8b379b203576`.
The relocated bundle's `lib/libsqlite3.dylib` SHA-256 was
`d7c3fa4c369d543ff35a5529d61a7e85f1914e1f0cc1ee07ee7f1d37f123562a`.
The difference was verified as install-name relocation plus ad-hoc re-signing;
owned copies were byte-identical after normalizing both changes. Matching
version strings alone would not establish this identity.

Separate [native VFS](db-native-storage.md) and [real ENOSPC](db-filesystem-full.md)
runs passed 132 and 23 tests respectively, using Dart 3.14.0 and the full-API
Apple **system** SQLite 3.54.0, source ID
`2026-04-09 12:25:13 8fa8248e303219400c646a885e36dfc52eae33d83f4412e3f369b2be5373aapl`.
The 32 MiB ENOSPC volume produced 22 actual failure cases. These runs used the
system engine explicitly; they do not claim native fault injection against the
packaged `be8d059e...` bundle tested above.

## Historical SDK evidence (before security upgrade)

Recorded on macOS arm64, 2026-10-15, after the separate
[#48](https://github.com/dustcodegen/dust/pull/48) compatibility fix:

| SDK | Standalone analysis / 7 tests / JIT / relocated AOT | Shopping analysis / 80 tests |
| --- | --- | --- |
| Stable Dart 3.10.0 archive | Passed | Not a Flutter SDK |
| Flutter 3.38.1 / stable Dart 3.10.0 | Passed | Passed |
| Flutter 3.44.1 / Dart 3.12.1 | Passed | Passed |
| Flutter 3.47.7 / Dart 3.13.5 | Passed | Passed |

Flutter 3.38.0 / prerelease Dart failed Pub resolution as described above; it is
not a skipped or passing execution target. The stable minimum passed separately.
Every successful JIT/AOT run loaded SQLite 3.53.4, source ID
`2026-07-24 19:02:57 bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc`.
All four bundles' `lib/libsqlite3.dylib` SHA-256 values matched:
`8a802ee4f58d4362bc36550c3faad3de17d2ce8304f3c825478ac5263c2dcdd4`.
These are host SDK/bundle results, not native mobile performance measurements.

That matrix pinned sqlite3 3.5.2 to compare native bytes with the recorded
Flutter proof. It does not validate the current 3.7.1 bundle or promise all
allowed binding versions behave alike. The current-bundle host results above
do not replace proof for other platforms or SDK combinations.
Runtime SQL in the standalone probe is unchecked; application SQL validation
uses [Dust and Rust SQLx](db.md), not runtime execution alone.

Full local `scripts/lint.sh`, `scripts/test.sh` and unfiltered **100% executable
SQLite runtime library line coverage** remain required. The SDK matrix does
not replace native I/O/ENOSPC, physical-device, iOS/Android lifecycle or local
[Linux](db-linux-validation.md)/Windows gates. No Actions run or version bump
is required.
