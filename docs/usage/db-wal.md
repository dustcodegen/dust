# SQLite WAL contention and checkpoints

WAL allows readers to retain a snapshot while another connection commits, but
still permits only one writer. Each Dust driver owns one connection, not a pool.
Choose `Sqlite3Driver.open` or `await Sqlite3WorkerDriver.open` per connection.
Prefer the worker for Flutter opening, migrations and SQL that might wait on locks.
See [SQLite WAL](https://www.sqlite.org/wal.html).

## Application usage

Run the temporary-file [checkpoint example](../../packages/dust_db_sqlite3/example/wal_checkpoint.dart):

```sh
cd packages/dust_db_sqlite3
dart run example/wal_checkpoint.dart
dart run example/wal_checkpoint.dart --background
```

Both modes print:

```text
reader snapshot: 1, checkpoint busy: 1
checkpoint complete: (0, 0, 0), current value: 2
```

The example opens two independent connections with WAL, `synchronous = FULL`,
zero busy timeout and automatic checkpoints disabled so the boundary is visible.
These are demonstration settings, not universal production defaults. SQLite's
default automatic checkpoint threshold is 1,000 pages; if you disable it, own
checkpoint scheduling and monitor WAL growth. Never remove live WAL/SHM files.

Use direct `queryExecute(...).execute(db)` or `.execute(tx)` calls. A reader
transaction retains its old snapshot even after the other connection commits.
After the reader finishes, a fresh query sees the new value. This example's SQL
and diagnostic PRAGMAs are unchecked runtime SQL, not Rust SQLx validation proof.
Use Dust's checked query/DAO build path with SQLx for application SQL validation.

## Inspect the checkpoint row

`PRAGMA wal_checkpoint(...)` returns one row with `busy`, `log` and
`checkpointed` integer columns. A returned `Ok` only means the query succeeded:

| Mode | What to inspect |
| --- | --- |
| PASSIVE | Does not wait for readers/writers. `busy = 0` can still mean only partial progress; compare frame counts. |
| FULL | Can wait for writers/readers; `busy = 1` means incomplete. |
| RESTART | Also waits for WAL readers so the next writer can restart the log. |
| TRUNCATE | Also shrinks the WAL to zero bytes when successful; all three returned values are zero. |

Frame counts can be `-1` if there is no WAL. Use a separate idle connection for
checkpoints, outside an active transaction. Keep reader transactions short and
schedule stronger checkpoints deliberately, away from latency-sensitive work.
See the [checkpoint contract](https://www.sqlite.org/pragma.html#pragma_wal_checkpoint).

`busyTimeout` controls SQLite lock waiting, not an operation-wide deadline.
Snapshot upgrades can fail immediately with `SQLITE_BUSY_SNAPSHOT`: waiting
cannot make an obsolete snapshot writable. End that transaction and re-read in
a new one before deciding whether to retry. Never automatically replay writes
or repeat external side effects from a transaction callback. Sync connections
block their isolate during native waits; a worker does not move result mapping
or application callbacks off the caller. See
[SQLite snapshot isolation](https://www.sqlite.org/isolation.html).

## Local regression evidence

Issue [#36](https://github.com/dustcodegen/dust/issues/36), under epic
[#19](https://github.com/dustcodegen/dust/issues/19), adds:

- 12 writer/snapshot cases across all four foreground/worker combinations,
  including zero and 20 ms busy timeouts, original error context, no partial
  writes and explicit fresh-transaction reuse.
- 40 checkpoint cases: four modes, held readers or active writers, all four
  connection combinations. They assert partial/busy rows, snapshot stability,
  progress after release, truncation, integrity, reuse and reader-case reopen.
  A fully copied log can still be busy for restart/truncation under a reader.
- Two worker wait cases with a native busy-handler handshake proving SQL and
  checkpoint lock waits leave the caller event loop responsive. The test-only
  handler is bounded; it does not replace the public busy-timeout tests.
- Four engine-provenance checks and two exact-output example tests.

```sh
dart test test/wal_contention_test.dart test/wal_checkpoint_test.dart \
  test/wal_worker_wait_test.dart test/wal_example_test.dart --reporter expanded
```

No sleeps guess lock state, no network or application retries are used, and all
databases are test-owned temporary files. This is host regression evidence, not
long-running load, hardware power-loss or physical Flutter device certification.

## Verify the native engine, not just the Dart package

The tests record `SELECT sqlite_version()` and `SELECT sqlite_source_id()` on
both connections. Local macOS validation reports **SQLite 3.54.0**, source ID
`2026-04-09 12:25:13 8fa8248e303219400c646a885e36dfc52eae33d83f4412e3f369b2be5373aapl`.
The Dart binding version is not the native SQLite version. This repository's
host hook uses the system library; record the actual loaded engine on every
supported target, including Android/iOS, before release.

SQLite documents a rare concurrent write/checkpoint WAL-reset corruption bug,
fixed in 3.51.3 and later, with backports in 3.44.6 and 3.50.7. Verify a fixed
release or confirmed vendor backport on each target. These deterministic tests
do not reproduce or certify that timing-sensitive upstream race and do not
silently skip tests on old engines. See the
[upstream WAL-reset advisory](https://www.sqlite.org/wal.html#walresetbug).
