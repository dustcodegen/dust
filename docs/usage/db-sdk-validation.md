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
access, and Puro only for the optional Flutter run. This runner was validated on
macOS arm64; it is not a Windows/Linux or mobile build certification.

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
disable them. Nine Python regression tests cover staging and proof validation:

```sh
python3 -m unittest discover -s scripts -p test_sqlite_sdk_consumer.py -v
```

## SDK boundaries

The package's declared floor remains **stable Dart >=3.10.0**, unchanged.
Flutter 3.38.0 actually bundles Dart `3.10.0-290.4.beta`; its Pub resolver rejects
that stable constraint before execution. Do not label this a passing floor test
or lower the package constraint to hide it. Flutter 3.38.1 bundles stable Dart
3.10.0 and is the tested stable floor build. This matches Flutter's
[upstream SDK packaging issue](https://github.com/flutter/flutter/issues/178400).
A separately downloaded stable Dart
3.10.0 archive was checked against its published SHA-256 before use.

Project Flutter 3.44.1 / Dart 3.12.1 and installed stable Flutter 3.47.7 /
Dart 3.13.5 are separate matrix targets. The exact commands/native proof, not a
nearby formatter patch, establish execution. The minimum shopping analyzer's
nullable-boolean diagnostics are tracked separately in
[#47](https://github.com/dustcodegen/dust/issues/47).

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

The fixture pins sqlite3 3.5.2 so native bytes remain comparable to the existing
Flutter proof. It does not promise all allowed binding versions behave alike.
Runtime SQL in the standalone probe is unchecked; application SQL validation
uses [Dust and Rust SQLx](db.md), not runtime execution alone. These source
features are unreleased despite unchanged numeric package versions.

Full local `scripts/lint.sh`, `scripts/test.sh` and unfiltered **100% executable
SQLite runtime library line coverage** remain required. The SDK matrix does
not replace native I/O/ENOSPC, physical-device, iOS/Android lifecycle or local
Linux/Windows gates. No Actions run or version bump is required.
