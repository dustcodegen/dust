# SQLite queries with or without a DAO

DAOs are optional. Write ordinary functions taking an `Executor`, or put the
query directly in an application function. No handwritten executor extension,
ORM or query-builder API is required.

```dart
Future<Result<List<Stock>, SqlxError>> loadStock(Executor db) {
  return queryAs<Stock>(
    'SELECT item, on_hand FROM stock ORDER BY item',
    [],
  ).fetchAll(db);
}
```

`Stock` uses `@Derive([FromRow()])`; import its library to bring the generated
typed terminals into scope. The same generated row decoder is used by a DAO.
See the [database setup](db.md#quick-start) for the database declaration and
migrations. A DAO is not required to define a database or validate queries.

## Choose execution per connection

```dart
final db = useBackground
    ? await AppDatabase.openBackground(path)
    : AppDatabase.open(path);
try {
  final stock = (await loadStock(db.connection)).unwrap();
  print(stock.length);
} finally {
  (await db.close()).unwrap();
}
```

For transactions, pass the supplied `tx` to the same function. Do not use the
parent connection or retain `tx` after its callback ends. Mappers and callbacks
still run on the caller isolate; background SQL results are materialized and
copied. Keep queries bounded.

Generated background opening and the query-only discovery fix described here
are scheduled for v0.3.0. Until it is published, use the matching current-source
CLI/runtime rather than the published 0.2.1 artifacts.

## Terminal behavior

| Query | Behavior |
| --- | --- |
| `queryAs<T>(...).fetchOne(db)` | Exactly one row; zero or multiple rows return a cardinality error. |
| `queryAs<T>(...).fetchOptional(db)` | Null for no rows; otherwise maps the first row. Use `ORDER BY` when selection matters. |
| `queryAs<T>(...).fetchAll(db)` | All mapped rows; an empty result is an empty list. |
| `queryScalar<T>(...).fetchOne(db)` | Exactly one scalar; nullable `T` also permits no row or SQL null. Multiple rows are an error. |
| `queryExecute(...).execute(db)` | Returns `ExecResult`, including affected rows and the driver's insert ID. |

`queryScalar<T>(...).fetchOptional(db)` uses nullable scalar decoding: no row or
SQL null returns null, while multiple rows still return a cardinality error.
This is different from the first-row behavior of row `fetchOptional`.

The [server parity tests](../../fixtures/server_app/test/query_parity_test.dart)
exercise both query styles on sync and worker connections, including empty and
multiple results, repeated placeholders, list binds, constraint failures,
connection reuse, commit/rollback, expired transaction scopes and close.

The [Flutter mapping tests](../../examples/shopping_app/test/sqlite_mapping_parity_test.dart)
cover generated JSON/converter/flattened/date mappings and malformed stored
values in all four combinations. Decode failures remain errors, not empty
results. Worker errors preserve category/driver/operation/constraint kind, but
their native cause is diagnostic text rather than the original native object.

## What Rust SQLx checks

Keep checked queries as literal SQL with literal bind lists in `lib/`:

```sh
dust build
dust db build
dust check --db --offline
```

DB mode now discovers Dust-aware query-only libraries even without a DAO or
another annotation in that file. It parses those libraries and validates their
query expressions against the package's migrations and row models. Normal
generation remains annotation-only. Query-only files do not need their own
generated part file. `example/` and `test/` entrypoints are not scanned as
application libraries; the runnable example calls checked functions in `lib/`.

Online SQLx validation rejects invalid SQL, bind counts and missing required
columns. It writes per-library query metadata during a successful DB build.
Offline checks require that metadata to match the SQL, migrations, driver,
terminal shape and row-column requirements. Missing or stale entries fail;
offline mode does not describe SQL against SQLite or rewrite the query cache.

Online row-type/nullability findings are **warnings**, not a full type-safety
proof. Today's offline cache records returned column names, not their type and
nullability metadata, so it does **not** replay those warnings. Runtime decode
and constraint errors remain possible in either mode. Direct executor SQL and
`unsafe` SQL are unchecked; running such SQL once does not validate it.

## Run the example

From `fixtures/server_app`:

The [example implementation](../../fixtures/server_app/lib/src/shared/db/checked_query_example.dart)
uses direct query calls ending in `execute(tx)` or `fetchOne(db)` /
`fetchOptional(db)` / `fetchAll(db)`. The entrypoint only opens and closes the
database. `Executor` is the shared query parameter type; passing a pool or a
connection does not change the call syntax, and the transaction uses `tx`.
Dust SQLite's current `Pool` compatibility interface still owns one connection;
it is not a multi-connection pool.

```sh
dart run example/checked_queries.dart
dart run example/checked_queries.dart --background
dart test test/query_parity_test.dart
```

Both execution modes print:

```text
one: shirt:5
optional: null
rows: 1
total: 5
```

These regressions establish the supported query contract, not physical-device,
process-crash, disk-full or production-load readiness.
