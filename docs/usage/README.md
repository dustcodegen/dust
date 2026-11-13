# Using Dust

These guides are for application developers. Complete the
[root quick start](../../README.md#quick-start), then follow the package section
for the feature you need.

Repository architecture, contributor validation, design notes, and release
evidence are grouped separately in [Developing Dust](../development/README.md).

## Dust CLI

- [Installation](../../README.md#installation)
- [CLI and package compatibility](./compatibility.md)

| Command | Behavior |
| :--- | :--- |
| `dust build` | Writes normal generated Dart outputs. |
| `dust check` | Checks normal generated outputs without writing. |
| `dust watch` | Runs an initial build, then rebuilds after source changes. Reports diagnostics as each build finishes. |
| `dust clean` | Removes Dust-owned generated outputs and cache state. |
| `dust doctor` | Reports workspace and plugin readiness. |
| `dust route table` | Prints the app route table without writing files. |
| `dust upgrade` | Checks, verifies, and upgrades the installed CLI binary. |
| `dust db build` | Validates static SQLite/PostgreSQL queries and writes database/DAO output. |
| `dust check --db` | Checks SQLite/PostgreSQL database and DAO output without writing. |
| `dust i18n scan` | Reports statically discoverable translation calls. |
| `dust i18n build` | Reconciles ARB files and refreshes the Flutter bootstrap. |
| `dust i18n check` | Validates ARB files and localization setup without writing. |

Run `dust --help` or `dust <command> --help` for current options. `--root`
accepts equivalent relative or absolute package paths, including pub workspace
members.

In a pub workspace, run `dust build` and `dust check` from a member package, or
pass `--root <member>`. Running either command at a root that declares
`workspace:` fails instead of succeeding without checking any libraries.

`dust upgrade --check` reads release metadata only. `dust upgrade --dry-run`
downloads and verifies the selected release without replacing the installed
binary. Package compatibility is tracked in the
[compatibility guide](./compatibility.md).

## `dust_dart`

| Task | Guide |
| :--- | :--- |
| Generate data-class helpers | [Derive](./derive.md) |
| Encode and decode JSON | [JSON serialization](./serde.md) |
| Validate models and Flutter forms | [Validation](./validation.md) |
| Generate Dio clients | [HTTP clients](./http.md) |
| Use Dart 3.13 primary constructors | [Primary constructors](./primary-constructors.md) |
| Validate SQL and generate DAOs | [Database](./db.md) |

See the [`dust_dart` package reference](../../packages/dust_dart/README.md) for
annotations and runtime APIs.

## `dust_flutter`

| Task | Guide |
| :--- | :--- |
| Build typed ViewModels | [State](./state.md) |
| Configure Navigator 2.0 routing | [Routing](./routing.md) |
| Add guarded routes | [Route guards](./routing-guards.md) |
| Handle incoming links | [Deep links](./routing-deep-links.md) |
| Keep independent branch stacks | [Shell routes](./routing-shells.md) |
| Maintain ARB translations | [Internationalization](./i18n.md) |

See the [`dust_flutter` package reference](../../packages/dust_flutter/README.md)
for runtime APIs.

## Database runtimes

- [`dust_db_sqlite3`](../../packages/dust_db_sqlite3/README.md): native SQLite
  runtime, opening modes, transactions, migrations, and WAL operation.
- [`dust_db_postgres`](../../packages/dust_db_postgres/README.md): PostgreSQL
  pool and executor runtime.
- [SQLite WAL guide](./db-wal.md): application configuration and operational
  boundaries.
- [SQLite query parity](./db-query-parity.md): generated DAO and inline query
  behavior.

## `dust_server`

The [server guide](../dust_server/README.md) covers routing, extraction,
responses, dependencies, middleware, authentication, WebSockets, rendering,
serving, tracing, security, and testing.

## Common workflows

For normal Dart and Flutter generation:

```bash
dust build
dust check
```

For Database projects:

```bash
dust build
dust db build
dust check
dust check --db
```

For translation maintenance:

```bash
dust i18n scan
dust i18n build
dust i18n check
```

> [!NOTE]
> Normal `dust build` does not generate `@SqlxDatabase` or `@SqlxDao` output.
> Database projects run `dust build` for row derives and `dust db build` for
> database and DAO generation.

## Runnable application examples

- [Product showcase](../../examples/product_showcase): Dart derives, JSON,
  validation, and HTTP generation.
- [Shopping app](../../examples/shopping_app): Flutter state, routing, i18n,
  HTTP, and Database integration.
