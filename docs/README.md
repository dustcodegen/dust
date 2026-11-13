# Documentation

Choose the path that matches what you are doing. Application setup and APIs
live under **Using Dust**. Repository architecture, validation, design work,
and release evidence live under **Developing Dust**.

## Using Dust

Start with the [root quick start](../README.md#quick-start), then use the guide
for the package you are adding to your application.

| Package | User documentation |
| :--- | :--- |
| Dust CLI | [Commands, workflows, and compatibility](./usage/README.md) |
| `dust_dart` | [Derive](./usage/derive.md), [primary constructors](./usage/primary-constructors.md), [JSON](./usage/serde.md), [validation](./usage/validation.md), [HTTP](./usage/http.md), and [Database](./usage/db.md) |
| `dust_flutter` | [State](./usage/state.md), [routing](./usage/routing.md), and [internationalization](./usage/i18n.md) |
| `dust_db_sqlite3` | [SQLite runtime](../packages/dust_db_sqlite3/README.md) |
| `dust_db_postgres` | [PostgreSQL runtime](../packages/dust_db_postgres/README.md) |
| `dust_server` | [Server guide](./dust_server/README.md) |

The [usage index](./usage/README.md) includes setup order, common CLI
workflows, package references, and runnable application examples.

## Developing Dust

Use the [developer documentation](./development/README.md) for:

- contributor setup and the local quality gate;
- engine architecture and plugin boundaries;
- package-by-package validation;
- active design work and retained decisions; and
- release and production-readiness evidence.

## Examples

| Example | Audience | Covers |
| :--- | :--- | :--- |
| [Product showcase](../examples/product_showcase/README.md) | Dart users | Derives, JSON, validation, and HTTP generation. |
| [Shopping app](../examples/shopping_app/README.md) | Flutter users | State, routing, i18n, HTTP, and Database integration. |
| [Benchmark project](../examples/benchmark_project/README.md) | Dust contributors | Build and cache performance measurements. |
