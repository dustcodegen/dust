# Independent SQLite consumer

This non-publishable fixture deliberately stays outside Dust's Pub workspace.
Its overrides use the actual source runtime packages and pin sqlite3 3.7.1,
which bundles SQLite 3.54.0. Recorded results from the earlier 3.5.2 binding are
historical. The [SDK/native-bundle guide](../../docs/usage/db-sdk-validation.md)
records the 2026-11-12 host results for the patched bundle; current-bundle
Android/iOS validation remains unverified. Standalone Dart 3.10.0 is supported;
Flutter SQLite consumers require Flutter 3.47.0 or newer.

From this directory, with the project SDK:

```sh
dart pub get
dart analyze --fatal-infos
dart test
dart run bin/sqlite_sdk_consumer.dart
```

The executable uses direct `queryExecute(...).execute(tx)` and typed reads on
sync/worker connections. It checks migration history, caller-side mapping,
BLOB/null values, committed data after close/reopen and integrity. Every handle
closes before its own temporary directory is deleted. A failure exits nonzero,
including AOT builds where assertions are disabled. Help exits 0; bad arguments
exit 64. No pool, handwritten query extensions or automatic write retries.

This runtime SQL is **unchecked** test instrumentation. For build-time query
validation use Dust's [SQLx-backed checked query/DAO workflow](../../docs/usage/db.md).
The existing generated Flutter shopping example supplies the checked bridges.

Run the additional [SDK/native-bundle gate](../../docs/usage/db-sdk-validation.md)
from the repository root. It builds with `dart build cli`, relocates the full
bundle, moves the staged source out of its original location, and executes from
an unrelated working directory. Shipping only the executable from that bundle
does not ship its SQLite native library. Normal local lint/test/format scripts
include this fixture; the multi-SDK and packaging gate remains explicit.
