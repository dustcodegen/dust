# CLI and Package Compatibility

Dust generated code depends on runtime APIs from the Dart and Flutter packages.
Keep the CLI and packages in a supported compatibility row.

The machine-readable source of truth is
[`compatibility/dust-cli-packages.json`](../../compatibility/dust-cli-packages.json).
CLI diagnostics, release checks, and docs should read from that contract instead
of duplicating version rules.

Run `dust doctor` to see the active CLI version, each Dust runtime package found
in `package_config.json`, the supported range, and whether the package is used
by the workspace source.

## Runtime SDK Floors

`dust_dart`, `dust_flutter`, `dust_db_postgres`, and `dust_server` support Dart
3.6 or newer. `dust_db_sqlite3` requires Dart 3.10 or Flutter 3.38 or newer,
matching the minimum SDK of its `sqlite3 ^3.3.0` dependency.

The floor is **stable** Dart 3.10.0: Flutter 3.38.0 ships a Dart prerelease
below that constraint. Flutter 3.38.1 supplies stable Dart 3.10.0. See the
[independent SDK/native consumer gate](db-sdk-validation.md) for exact versions,
isolated lint-tool resolution and packaging checks; no SDK constraints change.

## SDK versions versus source syntax

A newer Dart or Flutter SDK does not update Dust's bundled Tree-sitter grammar.
Supported handwritten syntax and generated Dart can work on a newer SDK even
when new language syntax is not supported by Dust. A language-version gate
records when Dart introduced a feature; it is not a parser-support guarantee.

Class-header primary-constructor support is in development under
[#76](https://github.com/dustcodegen/dust/issues/76). This branch exercises typed
declaring parameters through generated Dart and Flutter consumers; it is not
complete or released. See the [development usage and limits](primary-constructors.md).
For released binaries, keep traditional in-body constructors and explicit fields.
This does not require changing constructor calls in your application.

The [listed-upstream grammar migration](listed-grammar-migration.md) is staged.
This source checkout uses the pinned listed upstream after extraction parity;
released binaries are not changed by updating the source. Named concise
constructors and dot shorthands still get explicit unsupported-generation errors,
even though the new grammar parses them. Grammar acceptance is not codegen support.

When syntax cannot be parsed, `dust build` and `dust check` fail with a nonzero
exit code, source context and guidance to run `dart analyze`. If Dart accepts
the file, Dust's grammar may be the limitation; upgrading only the SDK will not
fix it. The guidance appears once per file, not on every parser error. Ordinary
malformed Dart still fails; the diagnostic does not claim it is valid.

A rejected library does not create or replace its generated output during a
normal build/check. Preserved output is stale, not evidence of successful
generation: fix the input and rerun both commands. Explicit `--clean` removes
generated files before building and does not promise to preserve them.

## Unreleased Background Opening

Generated SQLite `AppDatabase.openBackground` is a source-tree feature, not part
of the published 0.2.1 release. Opting in requires a CLI built from the change
and the matching `dust_db_sqlite3` source containing `Sqlite3WorkerDriver` and
`Sqlite3WorkerUnsafeSql`. Numeric version checks cannot distinguish these
unreleased APIs while versions are intentionally unchanged. Do not combine
opt-in generated code with the published 0.2.1 SQLite runtime or assume the
current release binaries generate the new opener. Ordinary synchronous-only
database declarations keep their existing generated output.

No versions or compatibility ranges are changed in this work. A coordinated
version/compatibility update is a required gate before publishing this feature.

## Dust CLI 0.2.1

| Dust CLI | `dust_dart` | `dust_flutter` | `dust_db_sqlite3` | `dust_db_postgres` | `dust_server` |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `0.2.1` | `>=0.2.1 <0.3.0` | `>=0.2.1 <0.3.0` | `>=0.2.1 <0.3.0` | `>=0.2.1 <0.3.0` | `>=0.2.1 <0.3.0` |

0.2.1 is a lockstep patch release. The CLI fixes migration-cache and pub
workspace discovery, both database runtimes read SQLx-style marked aliases,
and `dust_server` keeps nested API misses out of a root SPA fallback.

## Dust CLI 0.2.0

| Dust CLI | `dust_dart` | `dust_flutter` | `dust_db_sqlite3` | `dust_db_postgres` | `dust_server` |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `0.2.0` | `>=0.2.0 <0.3.0` | `>=0.2.0 <0.3.0` | `>=0.2.0 <0.3.0` | `>=0.2.0 <0.3.0` | `>=0.2.0 <0.3.0` |

0.2.0 is a minor bump rather than a patch because two things break across it.
Inline query terminals return `Result<T, SqlxError>` instead of throwing, and
the query cache format moved to version 3, so every committed `.dust_sql/` is
stale until `dust build --db` writes it again. A pubspec asking for `^0.1.4`
stops before 0.2.0, which is the point: an app takes both changes when it says
so rather than on its next `pub get`.

Every package carries the CLI's version, including `dust_flutter`, whose only
change this release was splitting `view_model.dart` into three files the library
still exports together. That would be a patch on its own, but all five packages
publish from one `v{version}` tag: a package on a different version cannot
publish, so the versions move together whether or not each one earned it.

## Dust CLI 0.1.4

| Dust CLI | `dust_dart` | `dust_flutter` | `dust_db_sqlite3` | `dust_db_postgres` | `dust_server` |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `0.1.4` | `>=0.1.5 <0.2.0` | `>=0.1.4 <0.2.0` | `>=0.1.5 <0.2.0` | `>=0.1.0 <0.2.0` | `>=0.1.0-beta.3 <0.2.0` |

`dust_server` is in the table because generated handlers call its API directly —
`Router.module`, `Route`, `Rejection`, `QueryExtractable`, `intoResponse` — so a
handler generated by the CLI does not compile against an older runtime. From
0.2.0 it carries the CLI's version rather than its own `0.1.0-beta` line: it is
still pre-1.0, and everything below 1.0 may still move.

## When Versions Do Not Match

- If the CLI is too old, install a newer Dust CLI release before generating.
- If a Dust package is too old, upgrade that package in `pubspec.yaml`.
- If a Dust package is newer than the CLI supports, upgrade the CLI first or pin
  the package back to the supported range.
- After changing Dust versions, run `dust clean`, `dust build`, and `dust check`
  so generated files match the active runtime packages.

> [!TIP]
> AI agents should check this file before editing Dust package constraints. Do
> not bump only one Dust package past the supported CLI row.

## Maintainer Rule

When a publishable package changes runtime source, public API, dependency
constraints, runtime behavior, or generated-code compatibility, bump that
package's `pubspec.yaml` version and update its `CHANGELOG.md` in the same PR.
If generated code needs the new package API, update
`compatibility/dust-cli-packages.json` too.

## Local SDK regression gate

Use the [minimum/latest Dart and Flutter consumer gate](sdk-validation.md) to
check fresh generation, strict analysis, execution and rejection of unsupported
syntax without changing versions, SDK defaults or remote CI. SQLite's higher
SDK floor and native/device validation remain separate gates.
