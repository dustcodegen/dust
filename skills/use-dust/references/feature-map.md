# Dust Feature Map

Use the project's installed package and CLI versions as the source of truth and
keep them in a supported compatibility row.

| Feature | Packages | Focused import | Package guide |
| :--- | :--- | :--- | :--- |
| Data classes, JSON, validation, HTTP clients | `dust_dart` | focused `dust_dart` library | [dust_dart](dust-dart.md) |
| Routing, state, i18n | `dust_flutter`, often `dust_dart` | focused `dust_flutter` library | [dust_flutter](dust-flutter.md) |
| SQLite Database | `dust_dart`, `dust_db_sqlite3` | `package:dust_dart/db.dart` | [dust_db_sqlite3](dust-db-sqlite3.md) |
| PostgreSQL Database | `dust_dart`, `dust_db_postgres` | `package:dust_dart/db.dart` | [dust_db_postgres](dust-db-postgres.md) |
| HTTP servers | `dust_server` | `package:dust_server/server.dart` | [dust_server](dust-server.md) |

`dust_server` is a Dart runtime on `shelf` whose API is modelled on Rust's
axum. It is not built on axum, and no Rust dependency is involved.

Install the CLI from the [Dust installation guide](https://github.com/dustcodegen/dust#installation).
Review remote installer commands before execution. Check CLI/package alignment
with the [compatibility guide](https://github.com/dustcodegen/dust/blob/main/docs/usage/compatibility.md).

## Authoring contracts

- **Data classes:** add `part 'model.g.dart';`, apply
  `@Derive([ToString(), Eq(), CopyWith()])`, and mix `_$Model` into the class.
- **JSON:** derive `Serialize()` and/or `Deserialize()`. Use `@SerDe` for
  options. Deserialization needs a forwarding
  `factory Model.fromJson(Map<String, Object?> json) => _$ModelFromJson(json)`.
- **Validation:** derive `Validate()` and add typed `@Validate(...)` field
  rules. Field annotations alone do not generate validation.
- **HTTP:** define an `abstract interface class` annotated with `@HttpClient`
  and a redirecting factory to `_$Client`. Annotate one HTTP verb per method
  and map parameters with `@Path`, `@Query`, `@Body`, and related annotations.
- **Routing:** create one `lib/route.dart` entrypoint that imports/exports
  `route/routes.g.dart`, extend the generated `$RootRouter`, and annotate route
  widgets with `@AppRoute`.
- **State:** annotate a ViewModel with `@ViewModel`, extend its generated base,
  keep UI values in state, and put dependencies in typed args.
- **i18n:** configure locales in `dust.yaml`, use literal namespaced keys,
  declare every locale asset directory, and wrap the app with generated
  `AppI18n`.
- **Database:** keep row models and database roots in separate libraries. Use
  `@Derive([FromRow()])`, `@SqlxDatabase`, `@SqlxDao`, raw static `@Query` SQL,
  and migrations. SQLite is native-only; it does not support web.
- **HTTP servers:** use the handwritten `Router` API when codegen is not
  wanted. Annotate a library with `@Routes` or a class with `@Controller`, add
  its `.g.dart` part, and run `dust build`.
  Handler bodies and deployment policy stay handwritten. Native-only; no web
  support.

## Command routing

| Task | Write command | Read-only verification |
| :--- | :--- | :--- |
| Normal `.g.dart` generation | `dust build` | `dust check` |
| Normal incremental generation | `dust watch` | `dust check` |
| Database row mapping | `dust build` | `dust check` |
| Database/DAO/SQL validation | `dust db build` | `dust check --db` |
| Offline Database validation | `dust db build --offline` | `dust check --db --offline` |
| Translation inventory | — | `dust i18n scan` |
| ARB/bootstrap reconciliation | `dust i18n build` | `dust i18n check` |
| Generated server modules | `dust build` | `dust check` |

An offline Database command requires compatible query metadata written by a
successful online database build. Normal `dust build` does not generate
`@SqlxDatabase`, `@SqlxDao`, or `@Query` output.

Firebase, Supabase, and ORM/query-builder behavior are not current Dust
features. Do not generate examples that imply otherwise.
