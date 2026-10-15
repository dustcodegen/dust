# SQLite crash/reopen recovery

Dust's local recovery tests kill a real database-owning process and reopen its
temporary file. They cover synchronous and persistent-worker connections with
both DELETE journals and WAL, using `synchronous = FULL`.

This is OS-process-crash evidence, not power-loss or complete production
certification. Database remains beta; the worker and generated background
openers are unreleased source features, not part of published 0.2.1 binaries.

Rust SQLx build-time validation checks SQL/schema shape, not crash durability.
Keep using `dust db build` and `dust check --db --offline`; these runtime
regressions complement those checks rather than replacing them.

## Tested boundaries

| Kill point | Expected after reopening |
| --- | --- |
| Inside an outer transaction, after SQL completes | Original committed rows remain; pending updates/inserts disappear. |
| Inside an active nested savepoint | Neither the nested nor outer pending writes survive. |
| After the transaction returns `Ok` | The acknowledged changes remain. |
| During migration SQL, after schema/data changes | No partial schema/data or new history entry remains. |
| At migration COMMIT, after history is recorded but before commit executes | Migration SQL and checksum history roll back together. |
| After managed opening returns | Migrated schema/data and exact checksum history remain. |

Every case checks `PRAGMA integrity_check`, exact data and SHA-256 migration
history, a subsequent successful transaction, and repeated managed startup.
Interrupted migrations can be applied from the original, unchanged bundle.
The test also forces pending pages out of SQLite's cache and verifies journal
bytes (and main-file growth in DELETE mode) before terminating the child.

There are 24 combinations. Each uses a separate test-owned database and a
compiled native Dart child. The parent waits for an atomic readiness marker,
then sends SIGKILL on Unix (process termination on Windows), without normal
close or rollback. Parent readiness and exit waits are bounded. Migration
instrumentation uses existing package-internal startup seams and native update/commit hooks;
normal application SQL and acknowledged opening use public drivers. No
production fault-injection hooks are added.

Run locally from `packages/dust_db_sqlite3`:

```sh
dart test test/process_crash_recovery_test.dart --reporter expanded
dart test --coverage=coverage
dart run coverage:format_coverage --lcov --in=coverage \
  --out=coverage/lcov.info --report-on=lib
python3 ../../scripts/check_lcov_floor.py coverage/lcov.info 100
```

The recorded local platform is macOS; Windows/Linux branches are not device or
platform validation until the same tests are run there. The test requires a
native Dart compiler and the project's configured SQLite native assets.

## Application recovery

Keep the database path and immutable migration bundle across launches. Open a
fresh connection after process death; normal managed startup verifies history
and applies only unapplied migrations. The same rule applies to
`AppDatabase.open(path)` and `await AppDatabase.openBackground(path)`.

If a live worker exits or its transport fails, the old executor is terminal.
Pending operations report connection errors; reopening does not resume their
callbacks. Do not automatically replay an uncertain write: it might have
committed before the result was lost. Reconcile using application-owned durable
operation IDs or inspect the intended state first. Line coverage and a clean
integrity check cannot resolve that application-level ambiguity.

When explicit full synchronization is appropriate for the application's
durability/latency tradeoff, it can be configured without changing query usage:

```dart
final db = await Sqlite3WorkerDriver.open(
  path,
  migrations: reviewedMigrations,
  options: const SqliteConnectOptions(
    journalMode: SqliteJournalMode.wal,
    synchronous: SqliteSynchronousMode.full,
    busyTimeout: Duration(seconds: 5),
    foreignKeys: true,
  ),
);
try {
  // Existing checked query helpers still take db; transactions take tx.
  final count = (await queryScalar<int>(
    'SELECT COUNT(*) FROM users',
    [],
  ).fetchOne(db)).unwrap();
  print(count);
} finally {
  (await db.close()).unwrap();
}
```

Keep the database and its WAL/SHM files together. Do not manually delete journal
files or copy only a live database file as a backup. Use a supported SQLite
backup/snapshot procedure. Migrations must leave transaction control to Dust.

## Controlled database-full recovery

The source runtime also handles SQLite automatically ending a managed
transaction after a native statement error. SQLite documents that errors such
as `SQLITE_FULL` can cause automatic rollback; the driver checks native
autocommit state rather than assuming every failed statement ends a transaction.
See [SQLite transaction errors](https://www.sqlite.org/lang_transaction.html).

When that happens, Dust retains the original statement error until the outer
callback finishes. Later reads, writes and nested transaction callbacks through
`tx` are rejected with that error; returning `Ok` cannot report success or
commit later writes outside the transaction. Throwing after the failure does
not hide the original error. Dust avoids rolling back nonexistent transactions
or savepoints, then clears the temporary error so the connection can be reused.
Actual cleanup failures still require closing and reopening. Errors that leave
the native transaction active keep their existing commit/rollback behavior.

The [transaction tests](../../packages/dust_db_sqlite3/test/disk_full_transaction_test.dart)
and [migration tests](../../packages/dust_db_sqlite3/test/disk_full_migration_test.dart)
cover 40 page-limit cases: sync/worker and DELETE/WAL, standalone statements,
returned/ignored/thrown transaction failures, nested savepoints, `RETURNING`,
attempted writes after failure, and managed migration startup. They verify
native `SQLITE_FULL` (code 13, diagnostic text on workers), exact original
data/history/timestamps, absence of partial schema and `integrity_check`.
The migration fault occurs in a later pending file, after an earlier pending
file's SQL and history insert have succeeded; both must roll back together.
Restoring test capacity permits an explicit retry; repeated startup verifies
migrations are applied only once. These fixes are unreleased source changes.

The tests use a small `PRAGMA max_page_count` on temporary, test-owned databases,
then attempt one bounded 256 KiB BLOB. This generates a real SQLite error without
filling the host disk. It limits database page growth, not the available storage
for WAL/journals or other files. See
[SQLite's page-limit pragma](https://www.sqlite.org/pragma.html#pragma_max_page_count).

Run the [safe demonstration](../../packages/dust_db_sqlite3/example/disk_full_recovery.dart)
from `packages/dust_db_sqlite3`:

```sh
dart run example/disk_full_recovery.dart
dart run example/disk_full_recovery.dart --background
dart test test/disk_full_transaction_test.dart test/disk_full_migration_test.dart \
  test/disk_full_example_test.dart
```

Both modes print:

```text
page limit: write failed, rows: 0
capacity restored: rows: 1
```

The example owns and removes its temporary database. Only that artificial cap
is restored; it is not a production strategy for removing quotas. A real app
must address its storage failure and reconcile uncertain writes before choosing
to retry. Dust never retries automatically. The example demonstrates runtime
recovery, not Rust SQLx build-time validation of its `example/` SQL.

## Still outstanding

Deterministic [WAL contention/checkpoint regressions](db-wal.md) cover held
readers/writers, snapshot conflicts and progress after release in sync/worker
combinations. They do not establish long-running load or the rare upstream
WAL-reset race; verify the loaded native engine on every supported target.

Controlled page-limit recovery is tracked in
[#33](https://github.com/dustcodegen/dust/issues/33) under epic
[#19](https://github.com/dustcodegen/dust/issues/19). Host-filesystem ENOSPC,
hardware power loss, storage corruption, Flutter device responsiveness,
long-running contention and production-load validation are not established by
these tests. Journal modes OFF/MEMORY and other synchronous modes are outside
this recovery matrix. No automatic retries, pool or backup API is added, and
these checks do not establish that all storage failures are recoverable.
