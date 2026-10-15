# SQLite recovery from actual filesystem ENOSPC

Issue [#40](https://github.com/dustcodegen/dust/issues/40), under SQLite epic
[#19](https://github.com/dustcodegen/dust/issues/19), adds actual filesystem
capacity failures. This is separate from [page-limit FULL](db-recovery.md) and
[injected native I/O faults](db-native-storage.md).

## Required explicit local integration gate

From `packages/dust_db_sqlite3` on macOS:

```sh
dart test integration_test/filesystem_full_test.dart --reporter expanded
```

Requirements: the native Dart compiler, `hdiutil`, `diskutil`, `plutil`, mount
permission and at least 512 MiB of free host space. Missing prerequisites fail
the invoked gate; nothing silently skips. No existing volume or application
database is accepted as a target. The default portable `dart test` suite does
not discover `integration_test/`; this command is an additional local gate.

## Why the host filesystem is not filled

The harness creates a fresh fixed-size **32 MiB HFS+ image** in a test-owned
temporary directory and mounts it without opening Finder. Before filler writes
it verifies that the image is attached at the expected mount point, on its
reported device, with the same UUID, HFS filesystem and bounded capacity.
There is also an independent 32 MiB write cap. Failed/missing identity or
capacity checks refuse to write. The portable safety tests exercise those
rejections without creating an image.

The filler writes only inside that verified image, until the OS reports
`ENOSPC` (errno 28). SQLite then attempts a bounded 256 KiB write/checkpoint.
On the recorded host every case returns native `SQLITE_FULL` (13), not a
synthetic page limit or an SQL/ownership failure. Test output records the
volume identity/capacity, filler bytes, observed OS/native errors and loaded
SQLite version/source ID. The image is never resized to consume the host disk.

## Regression matrix

| Scenario | Journal / execution | Cases |
| --- | --- | ---: |
| Outer transaction, nested savepoint, ignored error after automatic rollback, standalone write, managed migration | DELETE/WAL × foreground/worker | 20 |
| Acknowledged WAL commit followed by a checkpoint needing main-file growth | WAL × foreground/worker | 2 |

The matrix has **22 failure cases**, plus a mount/verified-detach smoke test.
Thirteen portable safety checks cover wrong/missing identities, filesystem and
capacity bounds. Each failure case owns a fresh image and a native Dart CLI
child built with its code-asset metadata through the shared bounded builder.
The parent waits for explicit atomic readiness/failure markers,
fills/restores the image and releases the child through stdin. No sleep guesses
when SQL has reached a boundary. Child and native-tool waits are bounded.

Migration instrumentation uses the existing internal startup seam to install
a test-only SQL function that pauses at the intended point. The parent, not
the callback, verifies/fills the volume. Production APIs and connection options
are unchanged; no native VFS injector is used in this gate.

The checks require the original error operation, no callback replay, exact
committed rows and nonzero-pattern BLOB bytes, unchanged migration checksum
history/timestamps, absent partial schema and `PRAGMA integrity_check` after
reopening. Returning `Ok` after the tested automatic rollback cannot commit a
following write. After deleting only the test filler, explicit retry succeeds;
repeated managed startup retains the same history. Previously acknowledged
WAL data survives the failed checkpoint and reopen. The new write is checked
again after a subsequent reopen.
Migration cases observe the earlier pending file's history insertion through a
synchronous native update hook before the later file pauses for filling. Both
pending files' schema/data/history must roll back, then apply once on explicit
retry with the unchanged bundle.

The parent waits for or terminates its child before closing the volume. Teardown
detaches only the device found through the exact owned image's attachment
record, verifies that the image is no longer attached, then removes the owned
temporary directory. If detach/verification fails, it fails the test and
preserves the image path for inspection rather than recursively deleting a
still-mounted volume. It does not force-detach unrelated disks or prune files.
An uncertain failed create/attach also preserves the backing image rather than
assuming a terminated command means all asynchronous OS work has finished.

## Application handling

Do not copy the filler/mount harness into an app. Run the safe
[page-limit recovery example](../../packages/dust_db_sqlite3/example/disk_full_recovery.dart)
to learn the direct `queryExecute(...).execute(db/tx)` and `Result` flow without
mounting a volume. Both synchronous and worker connections use the same query
and transaction APIs; neither retries automatically.

For a real storage failure, stop the affected operation, address storage and
reconcile uncertain writes before deciding whether to reopen/retry. End the
owning callback before closing its connection. Never replay an uncertain
transaction callback with external side effects. A failed statement does not
always end a native transaction; Dust's automatic-rollback guard follows actual
native state rather than assuming every error rolls back everything. Native
worker error codes in these tests are diagnostic strings, not a new structured
application error-code API.

Keep the database with its live WAL/SHM files; do not delete recovery files or
copy just the main live file as a backup. Rust SQLx build-time validation checks
SQL/schema shape, not available storage or durability; continue using Dust's
SQLx-backed checked query/DAO workflow for application SQL.

## Evidence limits

Recorded evidence is macOS HFS+ on a disposable image, `synchronous = FULL`,
SQLite 3.54.0 with source ID
`2026-04-09 12:25:13 8fa8248e303219400c646a885e36dfc52eae33d83f4412e3f369b2be5373aapl`.
This does not certify APFS, Android/iOS flash, Windows/Linux filesystems,
hardware power loss, all native I/O errors or all production loads. Other
filesystems can surface capacity failures differently. Physical-device and
cross-platform/SDK evidence remain separate completion gates in #19.

Root lint/test scripts, native I/O tests and unfiltered 100% executable SQLite
runtime library line coverage remain required. Passing this gate is not a
whole-epic readiness certificate or a release/version change.
