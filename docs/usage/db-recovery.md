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
close or rollback. Parent readiness and exit waits are bounded. Migration instrumentation
uses existing package-internal startup seams and native update/commit hooks;
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

## Still outstanding

Controlled `SQLITE_FULL` recovery is tracked separately in
[#33](https://github.com/dustcodegen/dust/issues/33) under epic
[#19](https://github.com/dustcodegen/dust/issues/19). Host-filesystem ENOSPC,
hardware power loss, storage corruption, Flutter device responsiveness,
long-running contention and production-load validation are not established by
these tests. Journal modes OFF/MEMORY and other synchronous modes are outside
this crash matrix. This change adds tests/documentation, not automatic retries,
a pool, backup APIs or a claim that all failures are recoverable.
