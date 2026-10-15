# SQLite native storage-fault validation

This is the injected native I/O phase of SQLite epic
[#19](https://github.com/dustcodegen/dust/issues/19), tracked in
[#38](https://github.com/dustcodegen/dust/issues/38). It complements
[process crashes and page-limit FULL](db-recovery.md) and
[WAL contention/checkpoints](db-wal.md). It is not hardware power-loss or actual
filesystem ENOSPC evidence; run the separate
[bounded-volume ENOSPC gate](db-filesystem-full.md) for that evidence.

## Run the required native integration gate locally

From `packages/dust_db_sqlite3`:

```sh
dart test integration_test/native_storage_fault_test.dart --reporter expanded
```

This explicit command is **additional to** root `scripts/lint.sh`,
`scripts/test.sh` and the unfiltered 100% SQLite runtime library coverage gate.
Default `dart test` discovers `test/`, not `integration_test/`.

Requirements: a macOS or Linux host, `clang`, system SQLite development headers
and link library, and a native Dart compiler. This repository's native-assets
hook selects system SQLite. The harness must register its VFS with that same
loaded engine; opening `dust_fault` and observing actual injected errors proves
registration reached it. Missing prerequisites or unsupported hosts fail the
explicit gate; tests never silently skip. Windows, Android and iOS native
storage-fault harnesses are not certified by a macOS run.

The shim needs the full public SQLite C API, including `sqlite3_vfs_find`.
Minimized bundled libraries may hide that symbol. Use a full-API engine for
this fault gate and record its provenance separately from bundled-consumer
checks; loading a second SQLite library does not register a VFS in the first.

## What is exercised

| Boundary | Matrix |
| --- | --- |
| Outer transaction, nested savepoint and managed migration | DELETE journal/database write and sync; WAL write and sync; foreground/worker; one-shot/persistent errors; whole/partial write failures. 108 cases. |
| WAL checkpoint | Database write/sync or WAL truncation; foreground/worker; one-shot/persistent; whole/partial database writes. 16 cases. |
| Database read | DELETE/WAL, foreground/worker, one-shot/persistent. 8 cases. |

Each of 132 cases runs in its own native Dart CLI bundle, built through
`dart build cli` so code-asset metadata is retained on Linux as well as macOS.
The shared bounded builder keeps its replaceable output below a separate
subdirectory, preserving the sibling VFS shim until all children finish.
The test-only C
VFS delegates normal I/O, locks and shared memory to SQLite's default native
VFS, conservatively advertises no atomic-write device capabilities, then returns
the selected real `SQLITE_IOERR_*` extended code at a specific
read/write/sync/truncate boundary. Partial-write cases write a real prefix
before returning `SQLITE_IOERR_WRITE`. The parent checks native fault counts
and the number of partial writes/bytes; a case cannot pass without injection.
The shim requires shared-memory-capable version-3 main-database file methods.
Journal/WAL handles can use version-1 methods; each wrapper advertises no more
than its native handle and the shim support, so unsupported callbacks are not
exposed as available.

The harness uses the native loader, not SQLite's disabled extension-loading
feature, and needs no handwritten Dart bindings. It selects the shim explicitly
only on test-owned connections; it never replaces the default VFS. Fault state
is passed through an atomically replaced control file in the test's temporary
directory. Configuration changes occur while the owning connection is idle.
Every spawned process has a bounded wait and is terminated on timeout. Handles
close before files are removed. Writes stay small; the host disk is never filled.

The C shim compiles with `-Wall -Wextra -Werror`; C and Dart integration fixtures
are included in the repository's 300-line source-size gate. No runtime fault
API, native binding, dependency, pool or automatic retry is added.

Assertions cover extended native errors (diagnostic text on worker transport),
operation context, exact committed rows/BLOB bytes and migration checksum
history/timestamps, absent partial schema and `PRAGMA integrity_check` after
reopening. Checkpoint failures must retain previously acknowledged data both
before and after reopening. Migration startup failures close the owned handle;
after disabling the test fault, unchanged migrations can be applied explicitly.
Fresh transactions work on reopened connections. One-shot and persistent fault
results are not treated as a guarantee that the original connection is reusable.

This follows SQLite's use of fault-injecting VFS layers, but does not duplicate
SQLite's exhaustive fault-position sweep, fuzzing or power-loss simulation.
See [SQLite I/O error testing](https://sqlite.org/testing.html#io_error_testing).

## Migration COMMIT errors retain their operation

Native I/O tests exposed a missing diagnostic: failures in migration control
statements retained their native cause but omitted `SqlxError.operation`.
Source now records the actual control boundary, such as `COMMIT`, without
changing message/category, replacing native causes or overwriting migration
filenames already supplied by lower-level validation/history errors.

Run [the safe COMMIT demonstration](../../packages/dust_db_sqlite3/example/migration_commit_failure.dart):

```sh
dart run example/migration_commit_failure.dart
dart run example/migration_commit_failure.dart --background
```

Both modes print:

```text
migration failed at: COMMIT
dependency repaired: rows: 1
```

It uses a deferred foreign key, not a native fault injector. The failed pending
migration leaves no partial row; the app reconciles the missing parent before
explicitly reopening with the same migration bundle. This diagnostic fix and
the new examples are unreleased source changes, not published 0.2.1 behavior.

Rust SQLx build-time validation still matters: it checks SQL/schema shape, not
native storage health or whether application data satisfies a deferred
constraint at commit. Use Dust's SQLx-backed checked query/DAO workflow for
application validation; these examples' runtime SQL is not build-time proof.

After a real storage error, address the storage problem and reconcile uncertain
writes before reopening/retrying. Do not replay a whole transaction callback
with external side effects automatically. Keep the database and WAL/SHM files
together, and never delete live recovery files to silence an error.
