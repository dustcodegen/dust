# Independent SQLite consumer

This non-publishable fixture deliberately stays outside Dust's Pub workspace.
Its overrides use the actual source runtime packages and pin the already-tested
sqlite3 3.5.2 binding. Worker opening is unreleased source functionality; these
unchanged numeric Dust versions do not make it available in published 0.2.1.

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
