# Package Production Readiness

Dust packages do not share one readiness claim. Stable and beta surfaces have
different compatibility promises, and repository checks cannot validate an
application's infrastructure or traffic.

## Current package status

| Package | Public status | Enforced local evidence | Boundary before deployment |
| :--- | :--- | :--- | :--- |
| `dust_dart` | Data classes, JSON, validation, and HTTP clients are stable. Database contracts are beta. | analyzer, package tests, and 100% line coverage | use a CLI version from the [compatibility contract](usage/compatibility.md) and review generated changes |
| `dust_flutter` | Routing and state are stable. i18n is beta. | analyzer, widget/runtime tests, at least 90% package coverage, and 100% router/state coverage | apps own platform links, host allow-lists, auth persistence, restoration policy, and device/browser validation |
| `dust_db_sqlite3` | Beta | analyzer, native tests, at least 96% line coverage, SDK consumers, and recovery probes | native only; validate the target filesystem, storage exhaustion, backup/restore, migrations, and power-loss policy |
| `dust_db_postgres` | Beta | analyzer and server-independent package tests | integration tests skip without `DUST_DATABASE_URL`; there is no enforced package coverage floor, so a real disposable PostgreSQL database is required evidence |
| `dust_server` | Runtime and generated wiring APIs are stable. | analyzer, runtime tests, generated consumers, and 100% line coverage | run the one-hour soak and test the real proxy, database, limits, secrets, supervision, and observability |

Coverage percentages above are enforced floors, not product-quality scores. Do
not copy changing test totals or measured line counts into public guides.

## Layered readiness gate

### 1. Repository gate

Run from the candidate source:

```bash
./.githooks/pre-commit
```

This is the authoritative local gate for formatting, analysis, security,
compatibility, tests, coverage, and generated-output drift. GitHub Actions are
reserved for release workflows, so an ordinary pull request does not prove the
local gate ran.

### 2. Package evidence

The complete gate already exercises packages separately. For a focused change,
run the owning package before the complete gate:

```bash
dart analyze --fatal-infos packages/dust_dart
dart test packages/dust_dart

flutter analyze packages/dust_flutter
flutter test packages/dust_flutter

dart analyze --fatal-infos packages/dust_db_sqlite3
dart test packages/dust_db_sqlite3

dart analyze --fatal-infos packages/dust_db_postgres
dart test packages/dust_db_postgres

dart analyze --fatal-infos packages/dust_server
dart test packages/dust_server
```

Focused success is not a substitute for the complete gate because generated
consumers cross package boundaries.

### 3. Release-candidate evidence

- Run the explicit minimum/latest [SDK matrix](usage/sdk-validation.md).
- Run the SQLite [SDK/native matrix](usage/db-sdk-validation.md) and the storage
  and recovery gates relevant to the supported hosts.
- Run PostgreSQL integration tests against a disposable database the test user
  may modify:

  ```bash
  cd packages/dust_db_postgres
  DUST_DATABASE_URL='postgres://user:pw@localhost:5432/dust_test?sslmode=disable' dart test
  ```

- Run the server's [release-candidate gate](dust_server/production-readiness.md#release-candidate-gate).
- Retain the SDK versions, operating systems, database versions, commands, and
  outputs used as evidence.

### 4. Application evidence

Before deployment, validate the real application rather than a package fixture:

- generated output is reviewed and `dust check` is clean;
- migrations, authorization, secrets, backup/restore, and dependency failures
  are exercised;
- sustained traffic uses the real payloads, middleware, database, proxy, and
  resource limits;
- supported browsers, devices, operating systems, and deep-link entry points
  are tested;
- logs, metrics, traces, alerts, process supervision, and rollback are
  configured and observed.

The accurate repository claim is that Dust has reproducible local quality gates
and documented compatibility boundaries. It does not certify every package
surface, application, or deployment as production-ready.
