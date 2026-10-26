# `dust_db_postgres` Guide

Use `dust_db_postgres` with `dust_dart` Database declarations for PostgreSQL on
native Dart or Flutter targets. It does not support web and is not an ORM or
query builder.

Keep raw SQL static and migrations append-only. Put `@Derive([FromRow()])` row
models in libraries separate from `@SqlxDatabase` and `@SqlxDao` roots. Use
`Executor` for queries so the same generated DAO can run against a connection
or transaction.

Treat connection URLs as secrets. Pass them through environment/configuration,
never generated source or fixtures. Choose TLS verification, connection
timeout, application name, and pool size explicitly for production. Keep the
combined pool size of all processes below the server's connection limit.

Run online validation with `DUST_DATABASE_URL=... dust db build`, followed by
`dust check --db`, `dart analyze`, and `dart test`. Use committed compatible
query metadata only for deliberate offline validation. Integration tests need
a disposable PostgreSQL database they are allowed to modify.

Use the facade's warning-bearing `unsafe` API only for SQL that cannot be
validated, such as administrative operations. Do not expose unsafe access
through request-scoped executors.

Source documentation:

- [Database usage](https://github.com/dustcodegen/dust/blob/main/docs/usage/db.md)
- [PostgreSQL runtime](https://github.com/dustcodegen/dust/blob/main/packages/dust_db_postgres/README.md)
- [Database query design](https://github.com/dustcodegen/dust/blob/main/docs/design/db-queries.md)
