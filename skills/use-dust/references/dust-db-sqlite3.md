# `dust_db_sqlite3` Guide

Use `dust_db_sqlite3` with `dust_dart` Database declarations for native Dart
or Flutter SQLite. It does not support web and is not an ORM, query builder, or
connection pool.

Keep raw SQL static so Dust can validate it. Put `@Derive([FromRow()])` row
models in libraries separate from `@SqlxDatabase` and `@SqlxDao` roots. Keep
migrations append-only: never edit, rename, or omit an applied migration.

Choose one connection per use case:

- `Sqlite3Driver` for direct synchronous native execution.
- `Sqlite3WorkerDriver` when open, migrations, queries, and close must run in a
  persistent background isolate. Await every boundary and close it explicitly.

Use `SqliteConnectOptions.memory` for tests. For production file databases,
review foreign keys, busy timeout, WAL, synchronous mode, read-only behavior,
legacy migration baseline policy, and bounded result sizes. Do not use an
explicit legacy baseline until the deployed migration SQL has been recovered
and reviewed.

Run `dust build`, `dust db build`, `dust check`, and `dust check --db`, then the
project's Dart or Flutter analysis and tests. Offline DB validation requires a
compatible metadata cache from a successful online build.

Source documentation:

- [Database usage](https://github.com/dustcodegen/dust/blob/main/docs/usage/db.md)
- [SDK validation](https://github.com/dustcodegen/dust/blob/main/docs/usage/db-sdk-validation.md)
- [Native storage validation](https://github.com/dustcodegen/dust/blob/main/docs/usage/db-native-storage.md)
- [SQLite runtime](https://github.com/dustcodegen/dust/blob/main/packages/dust_db_sqlite3/README.md)
