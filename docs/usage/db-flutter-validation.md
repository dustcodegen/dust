# Local Flutter SQLite lifecycle validation

This is the explicit device integration gate for
[#42](https://github.com/dustcodegen/dust/issues/42), under SQLite epic
[#19](https://github.com/dustcodegen/dust/issues/19). It uses the existing
shopping example's generated `ShoppingCacheDatabase.openBackground` facade.
It does not change normal app startup, contact live shopping endpoints, add a
pool or retry application writes.

## Android: two separate processes

Use a designated test device/emulator with no unrelated installation of
`com.example.shopping_app`. Installing a test build replaces that package's
code; never uninstall an existing app or clear its data just to run this gate.
Select the target explicitly with `adb devices -l` and `flutter devices`.

From `examples/shopping_app`, with Flutter and Android build prerequisites:

```sh
flutter pub get --offline
export DUST_SQLITE_ANDROID_SERIAL=emulator-5554
export DUST_SQLITE_ADB=adb
export DUST_SQLITE_PROBE_RUN="local-$(date +%Y%m%d%H%M%S)"
(
for phase in write verify; do
  export DUST_SQLITE_PROBE_PHASE="$phase"
  flutter drive --no-pub --profile --no-dds --keep-app-running \
    -d "$DUST_SQLITE_ANDROID_SERIAL" \
    --driver=test_driver/sqlite_lifecycle.dart \
    --target=integration_test/sqlite_lifecycle_test.dart \
    --dart-define="DUST_SQLITE_PROBE_RUN=$DUST_SQLITE_PROBE_RUN" \
    --dart-define="DUST_SQLITE_PROBE_PHASE=$phase" || exit 1
done
)
```

The two runs must use the same ID. A successful write leaves its unique test
database and ownership/history manifest in the app's application-support directory.
Verification requires an existing database and a **different OS process ID**;
it never silently replaces missing restart evidence with a fresh database.
Successful verification closes all connections and removes only that named
probe directory. A failed write preserves its files for inspection; choose a
new ID instead of overwriting them. `path_provider` supplies this private app
directory; code-cache/temporary storage is not appropriate for a relaunch or
app-update test.

Keep `--keep-app-running`: Flutter drive's default cleanup uninstalls the app,
which deletes its data. The next Android launch stops the previous process and
replaces the APK without clearing application-support data. The second phase
requires a different PID. The test installation remains afterward; stop it
without uninstalling if you want to inspect it. Never clear an unrelated app's
data. Android also documents that its
[code cache is cleared on app/platform upgrades](https://developer.android.com/reference/android/content/Context#getCodeCacheDir()).

The driver checks that the selected Android device runs the same probe process
before sending Home. It waits for an actual `paused` event, then launches the
same activity and requires an actual `resumed` event. These are OS transitions,
not injected binding messages. Keep the device unlocked during the test. A
missing handshake, target, native library or prerequisite fails the invoked
gate; nothing silently skips.

## What the probe checks

- A foreground connection holds a real writer lock before generated worker
  opening. A short native busy timeout must fail migration startup at
  `BEGIN IMMEDIATE`, without partial history. After release, an explicit
  generated-open retry applies both migrations once.
- A separate controlled worker-open fixture uses the existing internal startup
  seam to install a test-only bounded native busy handler. Its handshake proves
  contention before the test awaits three actual caller UI frames and releases
  the lock. Native timeout durations are not used as minimum-wait promises or
  guesses about SQL readiness. No hook or native handle is added to app APIs.
- Direct `queryExecute(...).execute(tx)` commits 128 rows. A bounded
  `queryAs(...).fetchAllWith(db.connection, mapper)` reads 32; the mapper asserts
  caller-isolate identity. There are no handwritten query extensions.
- The open worker retains exact committed rows across OS background/resume.
  Close/reopen cycles and a second process verify exact data, migration
  timestamps/checksums and `PRAGMA integrity_check`.
- The driver observes one `Dust SQLite` worker during the open connection and
  no remaining worker after all closes. RSS and raw frame build/raster samples
  are recorded with p50/p95/max summaries, not treated as a universal memory or
  frame-latency certificate.
- Evidence records OS, Dart/build mode, process identity and loaded native
  `sqlite_version()` / `sqlite_source_id()`. Android adds API level, build
  fingerprint and an emulator indicator.

The default host/widget test suite does not discover `integration_test/`.
Run this additional gate explicitly. The helper regressions are part of normal
`flutter test`, and source-size/strict-analysis/current-floor-format gates
include integration tests and their driver.

## Recorded local evidence

On 2026-10-15 the API 35 arm64 Android emulator passed two final fresh-ID,
two-process profile runs with Flutter 3.44.1 and Dart 3.12.1. Each checked 128
committed rows, bounded 32-row caller mapping, exact migration history,
background/resume, ten close/reopen cycles per phase, distinct process IDs and
zero remaining SQLite workers after close. The controlled native wait observed
three UI frames before lock release. SQLite 3.53.4 reported source ID
`2026-07-24 19:02:57 bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc`.

The native-packaging fix is merged in
[#44](https://github.com/dustcodegen/dust/pull/44). The downloaded arm64 binary
matches sqlite3 3.5.2's SHA-256 table; Gradle's stripped library matches the APK
entry. This proves the recorded bundled target, not every consumer resolution.
Full local lint/test gates and 529 combined SQLite tests passed, with unfiltered
963/963 executable runtime library lines covered. Existing environment-dependent
suite skips were not expanded. Floor formatting is not a minimum-SDK execution
result; supported-SDK consumers remain a separate #19 gate.

Probe development dependencies are Flutter SDK test tooling, `path_provider`
for durable test storage and an explicit reference to the already-resolved
sqlite3 binding for the internal native busy-handler fixture. No existing
resolved dependency version was upgraded; Dust runtime dependencies/APIs are
unchanged. The example's existing Kotlin/plugin DSL remains selected without
upgrading its build-tool versions; current Flutter still warns about their
future support window.

## Artifacts and limits

The host driver writes `build/sqlite-<run>-write.json` and
`build/sqlite-<run>-verify.json` through Flutter's standard integration-result
writer. Keep these ignored local artifacts with build logs when reviewing a
run; do not commit machine paths or private device metadata. Inspect the
reported native engine and confirm a fixed WAL-reset release or vendor patch;
the Dart binding version alone is insufficient.

On iOS, use a designated signed test installation and its explicit device ID.
Without the Android serial environment variable, the driver prompts for manual
Home/return transitions and applies the same observed-event deadlines. A
simulator/debug run is functional evidence only; physical-device/profile
evidence remains separate. This does not automate iOS signing or waive missing
device prerequisites.

An emulator's frame/RSS measurements are diagnostic, not physical Android
performance proof. Process relaunch is not hardware power loss. Tests do not
certify all OS suspension policies, low-memory kills, long-running workloads or
storage devices. Windows, supported SDK consumers and physical-device results
remain explicit #19 gates until run and recorded.

For application usage, follow [background opening](db.md#generated-background-opening-unreleased)
and the SQLx-backed checked query/DAO guide. The probe's runtime SQL is test
instrumentation; it does not turn arbitrary runtime SQL into checked SQL.

No package/version release or remote CI is required. Follow Flutter's
[integration profiling guide](https://docs.flutter.dev/cookbook/testing/integration/profiling)
for the profile-mode driver workflow.
