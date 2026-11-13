# Local Linux SQLite validation

This is the local Linux arm64 gate for
[#50](https://github.com/dustcodegen/dust/issues/50), under SQLite epic
[#19](https://github.com/dustcodegen/dust/issues/19). It reuses the
[independent consumer](db-sdk-validation.md) and existing recovery tests.
It does not invoke remote CI, publish packages or change production dependencies.

## Run

From the repository root, with local Docker, Python 3.10+, network access for
the pinned image/packages and adequate Docker disk headroom:

```sh
python3 -m unittest discover -s scripts -p 'test_sqlite*.py'
bash scripts/test_sqlite_linux.sh
```

The wrapper builds the official Dart 3.12.1 Linux arm64 image by digest and
installs test tooling. Only the disposable fault-test engine uses Debian's
[signed 2026-11-11 snapshot](https://snapshot.debian.org/archive/debian/20261111T000000Z/):
`libsqlite3-0` and development headers are pinned to `3.54.1-2`.
GPG signature verification remains enabled through Debian's archive keyring.
`Check-Valid-Until: no` allows this immutable snapshot after its metadata expires;
it does not disable signature verification. If those exact packages are
unavailable, the build fails; it does not silently substitute a different native
engine. Installed tool versions, image identity and source commit are retained
with the results.

The repository is mounted read-only. The runner copies unchanged package
manifests/source into owned staging, retaining workspace markers. Only that
temporary workspace selects the native engine and pins sqlite3 3.7.1. Its
default bundle contains SQLite 3.54.0; the fault phase uses the pinned system
engine instead. No source lock, cache or SDK default is overwritten.
The container has bounded CPU/memory/process counts, dropped capabilities and
no Docker socket, device, home-directory or privileged access. It is removed
after execution; ignored host evidence is retained under `.dart_tool/`.

## Two engines, different evidence

| Engine | Required execution |
| --- | --- |
| Default hash-verified SQLite bundle | Strict consumer analysis/formatting, seven tests, JIT and relocated AOT; all portable SQLite runtime tests, including crash/reopen and WAL contention. |
| Pinned full-API system SQLite | Strict runtime analysis, all portable tests and the complete 132-case native VFS I/O fault matrix. |

Linux's minimized bundle hides `sqlite3_vfs_find`, which the existing
[native fault shim](db-native-storage.md) needs. Loading another SQLite engine
does not register a VFS in the first. The system phase explicitly selects the
installed shared library for Dart and links the shim to that same package.
Native fault-hit and partial-byte assertions must pass. This does **not** claim
the shim exercised the stripped bundled binary.

The bundle's SHA-256 and actual loaded SQLite version/source must agree across
consumer JIT/AOT and all four sync/worker WAL connection pairs. The system phase
selects the SONAME from the package-owned file list, records its SHA-256 and
requires matching native version/source from all four actual Dart pairs.
A CLI's version alone is insufficient: the CLI can link SQLite statically.
Both engines must contain a fixed [WAL-reset implementation](https://sqlite.org/wal.html);
unknown vendor backports require separate evidence.

Unfiltered executable runtime library coverage combines both phases and must
meet 100%, without ignores, exclusions or threshold reductions. A fresh second
container then runs the relocated consumer bundle **offline**, with no source,
Pub cache or staging from the first container. Its native/data/migration proof
must match; only independently created database timestamps may differ.

## Patched engine evidence (2026-11-12)

With sqlite3 3.7.1 and Dart 3.12.1 on Linux/aarch64, the prepared configuration
passed strict consumer formatting/analysis, seven consumer tests, JIT and
relocated AOT, 375 bundled-engine portable tests, and 507 full-API tests
(375 portable + 132 native VFS I/O). Unfiltered executable runtime coverage
was **965/965 (100%)**, with no new skip or exclusion.

The relocated consumer also passed in a fresh, network-disabled container with
no source, Pub cache or staging from the first container. Native/data/migration
facts matched the first run.

| Engine | SQLite | Library SHA-256 |
| --- | --- | --- |
| Bundle | 3.54.0 | `c2858e7048931194b4acd71258ac36bdb7c881c697b1d972b7206142bdaa68f6` |
| Debian full API, package 3.54.1-2 | 3.54.1 | `afb264d5392f72416a29d91a01d49d3c0a9e7c6127e66f0a2156d2ad2b4ca52d` |

The bundled bytes exactly matched sqlite3 3.7.1's published Linux arm64 asset
hash. The native VFS fault matrix exercised only the separate full-API system
engine; the bundle's portable/runtime and offline AOT results are distinct.

## Historical pre-upgrade functional evidence (2026-11-12)

With the unchanged sqlite3 3.5.2 binding and Dart 3.12.1 on Linux/aarch64,
the signed-snapshot harness passed seven consumer tests, strict analysis and
formatting, JIT and relocated AOT. It also passed 882 runtime test executions:
375 portable tests on bundled SQLite 3.53.4, then 507 tests on the separate
full-API system SQLite 3.54.1 (375 portable + 132 native VFS I/O).
Unfiltered executable runtime coverage was **965/965 (100%)**.

The relocated consumer passed again in a fresh, network-disabled container
without source, Pub cache or staging from the first container. Native, data and
migration facts matched. The native VFS fault matrix exercised the system
engine only. This functional run does not validate a patched SQLite 3.54.x
bundle; its bundled engine remained SQLite 3.53.4.

## Historical local evidence (2026-10-15)

On 2026-10-15, Dart 3.12.1 on local Linux/aarch64 passed the independent
consumer's seven tests/JIT/relocated AOT, 375 bundled-engine portable tests,
507 full-API tests (375 portable + 132 native I/O) and the fresh offline
container proof. Unfiltered runtime library coverage was **963/963 (100%)**.
The five cold-formatter, seven Linux-runner and nine consumer checks passed
locally too; no new skip or coverage exclusion was introduced.

Both engines reported SQLite 3.53.4, but they are distinct native builds:

| Engine | Library SHA-256 |
| --- | --- |
| Bundle | `7f0db19fdb987298ab8def51431aba5cec28a81685e8b62cebbc7402df090a16` |
| Debian full API | `4ec9a6f5ec476bf26112a744350696aa711c35c5a263476e95e8cc8001810442` |

Bundled source ID:
`2026-07-24 19:02:57 bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc`.
Full-API source ID:
`2026-07-24 19:02:57 bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e0945alt1`.
Recorded tools were Python 3.13.5-1, clang 1:19.0-63 and binutils 2.44-3;
both SQLite packages were 3.53.4-2. Matching version strings alone do not
establish identical bytes, compilation flags or fault coverage.

## Evidence and boundaries

Each run retains command logs, resolved staging locks, engine/version/source
and native hashes, raw coverage, `runtime.lcov`, `result.json`, the consumer
bundle and the fresh-container proof. Missing prerequisites, provenance,
coverage or assertions fail the explicit gate; nothing silently skips.

This is Linux arm64 container/VM evidence, not Linux x64, Windows, mobile
Flutter or physical-storage certification. The bounded HFS+ real ENOSPC gate
remains [macOS-only](db-filesystem-full.md); it is not invoked or labelled as
passing Linux evidence. Process kills and injected I/O are not hardware
power-loss tests. Physical-device/lifecycle checks remain separate in
[#42](https://github.com/dustcodegen/dust/issues/42).

Runtime probe SQL is unchecked. Application SQL still uses Dust's
[Rust SQLx-backed query/DAO validation](db.md). Full host lint/test scripts and the
final epic audit remain required in addition to this Linux gate.
