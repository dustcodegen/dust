# Database

Dust Database generates typed SQLite and PostgreSQL access from raw SQL and
validates static queries against your migrations with Rust SQLx. It is not an
ORM or query builder.

Database is currently beta. SQLite uses `dust_db_sqlite3`; PostgreSQL uses
`dust_db_postgres`. This guide uses SQLite for its main walkthrough because it
can run locally without a database server. The
[PostgreSQL runtime guide](../../packages/dust_db_postgres/README.md) documents
pooling, migrations, transactions, arrays and production configuration.

## Add the Packages

Install the Dust CLI from the [root guide](../../README.md#installation), then
add the annotations and the runtime your application uses:

```bash
dart pub add dust_dart dust_db_sqlite3
```

For PostgreSQL:

```bash
dart pub add dust_dart dust_db_postgres
```

The SQLite package uses native libraries through Dart FFI and is intended for
native Dart and Flutter targets, not web. PostgreSQL is also native-only and
connects to a server over a socket.

Execution is opt-in per connection. Existing `Sqlite3Driver` and generated
`AppDatabase.open` remain synchronous. For native opening, migration checks,
SQL and close on a persistent isolate, use
`await Sqlite3WorkerDriver.open(path, migrations: reviewedMigrations)` and pass
the returned executor to your generated DAO. Row mapping and callbacks stay on
the caller; direct executor SQL remains unchecked. Static `queryAs`,
`queryScalar` and `queryExecute` calls can still be validated without a DAO. See the
[background connection guide](../../packages/dust_db_sqlite3/README.md#optional-background-connection)
for usage, ownership, failure recovery and limits. Generated async opening is
an explicit [opt-in](#generated-background-opening).

For ordinary functions with an explicit executor, all five terminals, and the
online/offline validation boundary, see [SQLite query parity](db-query-parity.md).
No handwritten executor extension or DAO declaration is required.

See [crash/reopen recovery](db-recovery.md) for local transaction/migration
recovery evidence and application handling of uncertain writes.
See [WAL contention and checkpoints](db-wal.md) for snapshot conflicts,
checkpoint status, foreground/worker usage and native-engine verification.
See [native storage-fault validation](db-native-storage.md) for I/O boundaries,
the extra local integration gate and migration COMMIT diagnostics.
See [actual filesystem ENOSPC recovery](db-filesystem-full.md) for the separate
bounded-volume gate and its host/filesystem limitations.
See [local Flutter lifecycle validation](db-flutter-validation.md) for the
explicit worker/OS/relaunch probe and device-evidence boundaries.

## Quick Start

Create a simple migration at `migrations/0001_create_users.sql`:

```sql
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL
);
```

Define row models in their own Dart library:

```dart
import 'package:dust_dart/db.dart';

part 'user_row.g.dart';

@Derive([FromRow()])
@Sqlx(renameAll: SqlxRename.snakeCase)
final class UserRow {
  const UserRow({
    required this.id,
    required this.email,
    required this.name,
  });

  final int id;
  final String email;
  final String name;
}
```

Define the database and DAO in `app_database.dart`:

```dart
import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';

import 'user_row.dart';

part 'app_database.g.dart';

@SqlxDatabase(
  type: SqlxDatabaseType.sqlite,
  migrations: './migrations',
)
abstract class AppDatabase implements DatabaseClient {
  factory AppDatabase.open(
    String path, {
    SqliteConnectOptions? options,
  }) = _$AppDatabase.open;

  @override
  Connection get connection;
}

@SqlxDao()
abstract final class UserDao {
  const factory UserDao(Executor db) = _$UserDao;

  @Query(r'SELECT id, email, name FROM users WHERE id = $1')
  Future<Result<UserRow?, SqlxError>> findById(int id);

  @Query(r'SELECT id, email, name FROM users ORDER BY name')
  Future<Result<List<UserRow>, SqlxError>> listUsers();

  @Query(r'SELECT COUNT(*) FROM users')
  Future<Result<int, SqlxError>> countUsers();

  @Query(r'INSERT INTO users (email, name) VALUES ($1, $2)')
  Future<Result<ExecResult, SqlxError>> createUser(
    String email,
    String name,
  );
}
```

Generate row mappers, then validate SQL and generate the database code:

```bash
dust build
dust db build
```

Open the database and pass its connection to a DAO:

```dart
final database = AppDatabase.open('app.db');
final users = UserDao(database.connection);

final result = await users.findById(42);
result.match(
  ok: (user) => print(user?.name),
  err: (error) => print('Database error: $error'),
);

await database.connection.close();
```

> [!IMPORTANT]
> Keep row models and database roots in separate libraries. `dust build` owns
> normal derives such as `FromRow`; `dust db build` owns `@SqlxDatabase`,
> `@SqlxDao`, and `@Query` output.

## Generated Background Opening

This feature is available in 0.3.0. Keep the CLI and SQLite runtime in the same
supported compatibility row. Existing synchronous-only declarations generate
the same output as before. See
[compatibility](compatibility.md#background-opening).

Add a static forwarding method to your SQLite database declaration:

```dart
@SqlxDatabase(type: SqlxDatabaseType.sqlite, migrations: './migrations')
abstract class AppDatabase implements DatabaseClient {
  factory AppDatabase.open(
    String path, {SqliteConnectOptions? options}
  ) = _$AppDatabase.open;

  static Future<AppDatabase> openBackground(
    String path, {SqliteConnectOptions? options}
  ) => _$AppDatabase.openBackground(path, options: options);

  @override
  Connection get connection;
}
```

Dart constructors cannot be async. This static method explicitly opts the
SQLite facade into generating a worker opener; an instance method or a
PostgreSQL declaration does not. Run both generation modes:

```bash
dust build
dust db build
dust check --db
```

Choose the connection mode when opening:

```dart
final app = await AppDatabase.openBackground(
  'app.db',
  options: const SqliteConnectOptions(
    foreignKeys: true,
    busyTimeout: Duration(seconds: 5),
  ),
);
try {
  final users = UserDao(app.connection);
  final result = await users.findById(42);
  final count = await queryScalar<int>(
    'SELECT COUNT(*) FROM users', [],
  ).fetchOne(app.connection); // No DAO required.
  print((result, count));
} finally {
  (await app.close()).unwrap();
}
```

Both openers embed exactly the same migration bundle. The future resolves only
after worker setup and migration verification/application finish; no app-side
migration file I/O is required. `migrate()` remains a no-op after SQLite startup.
The checksum and reviewed legacy-baseline policies are unchanged. A generated
facade always manages its embedded bundle, including an empty one; for unmanaged
or read-only inspection, open a runtime driver without migrations instead.

`connection`, `executor`, `pool`, transactions and close retain their contracts.
The background connection is not a `Sqlite3Executor` and has no native handle:
do not cast it for `.database`. `unsafe.fetch`, `fetchAs` and `execute` remain
asynchronous, use a worker-safe adapter, and enforce transaction ownership.
Unchecked SQL still needs an explicit mapper and is not SQLx-validated. Inside a
transaction use its `tx`, never the root facade or root `unsafe` adapter.

Row mapping and callbacks stay on the caller. Results are materialized and copied;
bound query results, await nested transactions, and reconcile uncertain writes
after a worker failure rather than replaying them. This is not a pool or a web
implementation. See the [Flutter regression](../../examples/shopping_app/test/sqlite_background_facade_test.dart)
and the [runnable server example](../../fixtures/server_app/example/background_database.dart).

## Migrations

Dust reads migration files in the configured directory in sorted filename order.
DB generation embeds the files that should run during normal startup. At
runtime, SQLite records applied filenames and exact UTF-8 SQL SHA-256 checksums
in `__dust_schema_migrations`. It rejects changed or missing applied files before
running new SQL, complementing Rust SQLx's build-time checks. Dust's runtime
history is separate from SQLx's `_sqlx_migrations` table.

Simple forward migrations use plain `.sql` names:

```text
migrations/
  0001_create_users.sql
  0002_add_user_avatar.sql
```

SQLx reversible migrations are also supported:

```text
migrations/
  0001_create_users.up.sql
  0001_create_users.down.sql
  0002_add_user_avatar.up.sql
  0002_add_user_avatar.down.sql
```

Example reversible pair:

```sql
-- migrations/0002_add_user_avatar.up.sql
ALTER TABLE users ADD COLUMN avatar_url TEXT;
```

```sql
-- migrations/0002_add_user_avatar.down.sql
CREATE TABLE users_new (
  id INTEGER PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL
);

INSERT INTO users_new (id, email, name)
SELECT id, email, name FROM users;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;
```

For reversible pairs, Dust validates the pair but applies and embeds only the
`.up.sql` file during normal build and runtime startup. `.down.sql` files are
never applied automatically. Every `.up.sql` file must have a matching
`.down.sql` file, and orphan `.down.sql` files are rejected.

Do not mix `0001_name.sql` with `0001_name.up.sql` or `0001_name.down.sql`.
Use one migration style per migration name.

> [!IMPORTANT]
> Do not edit or rename a migration after it ships. Existing installations have
> already recorded its filename; add a new migration for every schema change.

SQLite managed startup acquires a writer lock with `BEGIN IMMEDIATE`; use
`busyTimeout` to control contention waits. The history upgrade and all new SQL
are atomic. Omitted/null migrations mean unmanaged startup, while an explicit
empty map still verifies history and requires write access.
Leave transaction control to Dust: native commit filtering prevents migration
SQL from committing startup early or committing implicit writes after rollback.
A boundary savepoint also detects a rollback followed by a replacement transaction.
This is not a sandbox for arbitrary SQL or a crash-safety guarantee for unsafe
journal settings or attached databases.

Old name-only SQLite history fails closed: back up the database and recover and
review the deployed SQL before opening once with
`SqliteConnectOptions(baselineLegacyMigrations: true)`. Generated database
openers already forward these options. Remove this opt-in after upgrading.
Baseline rows retain their timestamps and are labeled `legacy-baseline`, not
historically verified; subsequent SQL is labeled `applied`. The opt-in cannot
override missing migrations, changed checksums or corrupted applied metadata.
See the [upgrade guide](../../packages/dust_db_sqlite3/README.md#existing-name-only-history)
and [executable example](../../packages/dust_db_sqlite3/example/legacy_migration_baseline.dart).

## SQLite Connection Options

`SqliteConnectOptions` keeps production connection behavior explicit without
exposing `package:sqlite3` open-mode types in application code.

Common path-based production settings:

```dart
final database = AppDatabase.open(
  'app.db',
  options: const SqliteConnectOptions(
    foreignKeys: true,
    busyTimeout: Duration(seconds: 5),
    journalMode: SqliteJournalMode.wal,
    synchronous: SqliteSynchronousMode.normal,
  ),
);
```

In tests, use an in-memory database without creating files:

```dart
final database = AppDatabase.open(
  ':memory:',
  options: const SqliteConnectOptions.memory(foreignKeys: true),
);
```

For an existing read-only file, open without migrations:

```dart
final connection = Sqlite3Driver.connect(
  SqliteConnectOptions.readOnly('snapshot.db'),
);
```

Supported options include create-if-missing, read-only open mode, busy timeout,
foreign keys, journal mode, synchronous mode, and custom pragmas. Custom pragma
names are validated before opening the database.

## DAO Return Types

The success type inside `Future<Result<T, SqlxError>>` selects the generated
executor call:

| Success type | Generated behavior |
| :--- | :--- |
| `RowType` | Requires exactly one row and maps it with `FromRow`. |
| `RowType?` | Returns zero or one mapped row. |
| `List<RowType>` | Maps every returned row. |
| `String`, `int`, `double`, `num`, `bool`, or `DateTime` | Reads one scalar from column zero. |
| `ExecResult` | Executes the statement and returns affected rows and last insert ID. |
| `Unit` | Executes the statement and discards execution metadata. |

DAO query methods must be abstract, return the exact `Future<Result<...,
SqlxError>>` shape, and use required positional parameters.

A DAO cannot return `List<Row>`. Untyped rows come from the facade's
[`unsafe`](#unchecked-sql) escape hatch, which a DAO's executor cannot reach.

## SQL Placeholders

Write checked `@Query` SQL with SQLx-style placeholders:

```dart
@Query(r'SELECT id, email, name FROM users WHERE id = $1 OR owner_id = $1')
Future<Result<List<UserRow>, SqlxError>> byIdOrOwner(int id);
```

For SQLite, Dust rewrites `$1` to `?` and repeats the matching Dart argument
when a placeholder appears more than once. Placeholder numbers must match the
DAO method's positional arguments.

Both simple and complex static SQL belong in `@Query`; CTEs, joins, grouping,
ordering, limits, and offsets are passed to SQLite and SQLx as written.

## Row Mapping

`@Derive([FromRow()])` generates three things, following the same naming the
other derives use — a private conversion function, a public witness, and the
query terminals:

```dart
Order _$OrderFromRow(Row row) { ... }

final class $OrderRowDeserializer implements RowDeserializer<Order> { ... }

extension $OrderQuery on QueryAs<Order> {
  Future<Result<Order, SqlxError>> fetchOne(Executor db) =>
      fetchOneWith(db, _$OrderFromRow);
  Future<Result<Order?, SqlxError>> fetchOptional(Executor db) => ...;
  Future<Result<List<Order>, SqlxError>> fetchAll(Executor db) => ...;
}
```

`_$OrderFromRow` mirrors serde's `_$OrderSerialize`, and `$OrderRowDeserializer`
mirrors `$OrderSerializer`. `RowDeserializer<T>` is the row-side mirror of
`Deserializer<DartT, JsonT>` and exists for the same reason: `serialize()` can
be an instance method because the value already exists, while reading a row
*constructs* one, so there is no instance to declare it on.

Generated DAO methods decode through the witness, so generated database code
depends on neither side-effect registration nor import order.

`@Sqlx` supports these mapping options:

| Option | Behavior |
| :--- | :--- |
| `renameAll` | Applies a naming rule to every field in the row class. |
| `rename` | Maps one field to an explicit column name. |
| `flatten` | Builds a nested `FromRow` type from the same row. |
| `skip` | Ignores a field; the field or annotation must supply a default. |
| `defaultValue` | Uses a value when the selected column is null. |
| `json` | Decodes a text column through `Type.fromJson(...)`. |
| `tryFrom` | Decodes a database value with a `SqlxTryFrom` converter. |

Directly supported field types are `String`, `int`, `double`, `num`, `bool`,
`DateTime`, and nullable variants. PostgreSQL rows also support `List<T>` of
those types, including nullable lists. SQLite has no array type; use a child
table, `json`, or `tryFrom` there.

For typed queries outside generated DAOs, write the query and call the terminal.
Nothing is passed:

```dart
final user = await queryAs<UserRow>(
  'SELECT id, email, name FROM users WHERE id = ?',
  [id],
).fetchOne(database.connection);

switch (user) {
  case Ok(:final value):
    print(value.email);
  case Err(:final error):
    print('lookup failed: $error');
}
```

Every terminal returns `Result<T, SqlxError>`, matching generated DAO methods
and the rest of `dust_dart`. A failed query is a value to handle, not an
exception to catch.

Dart resolves an extension member from the **static type** of the receiver, so
`fetchOne` here is `$UserRowQuery.fetchOne`, picked at compile time. A row type
with no `@Derive([FromRow()])` has no terminals at all, so the call does not
compile — no registry lookup, no global state, no argument. `dust db build`
reports it first, with a better message:

```
error: queryAs<Widget> row type has no row mapping. Add `@Derive([FromRow()])`
       to `Widget`, or pass one with `fetchOneWith`
```

For a row type Dust does not generate, the `With` terminals take the mapping:

```dart
final row = await queryAs<Legacy>(
  'SELECT id FROM legacy WHERE id = ?',
  [id],
).fetchOneWith(database.connection, Legacy.fromRow);
```

If the row library uses a `show` clause on `package:dust_dart/db.dart`, it needs
`QueryAs`, `Executor`, `Row`, `RowDeserializer`, `Result`, and
`SqlxError` for the generated part to compile.

## Error Context

DB calls return `Result<T, SqlxError>`. The error string stays short for app
developers, and the error also carries structured fields for logs and tests:

- `category`, such as `connection`, `migration`, `query`, `decode`,
  `cardinality`, or `transaction`
- `driver`, when known
- `operation`, such as the SQL string, migration name, or transaction command
- `cause`, when the lower-level driver provided one
- `kind`, when a statement broke an integrity constraint: `uniqueViolation`,
  `foreignKeyViolation`, `notNullViolation`, or `checkViolation`. SQLite and
  PostgreSQL both fill it from their own error codes, so matching on it works
  the same on either database, where matching on message text does not.

```dart
switch (await users.insert(email)) {
  case Ok(:final value):
    return value;
  case Err(error: SqlxError(kind: SqlxErrorKind.uniqueViolation)):
    return const Err('that email is already registered');
  case Err(:final error):
    return Err('could not save: ${error.message}');
}
```

Generated DAOs and the SQLite runtime use these categories for common failures:
missing tables, migration errors, decode failures, wrong row counts, closed
connections, and transaction control failures.

## Validation

`dust db build` applies migrations to an in-memory SQLite database by default,
asks SQLx to describe each static query, writes generated Dart, and caches query
metadata in one file per library:

```text
.dust_sql/<library>-<hash>.json
```

It validates migration SQL, placeholders, static query syntax, scalar column
count, known row columns, supported return types, and DAO method shapes.

Use a no-write check in CI:

```bash
dust check --db
```

Set `DUST_DATABASE_URL` only when validation must use another SQLite database.

> [!WARNING]
> DB validation applies the configured migrations to `DUST_DATABASE_URL`. Never
> point it at a production or user database.

## Offline Validation

After a successful online `dust db build`, validation can reuse matching cached
metadata without opening a validation database:

```bash
dust db build --offline
dust check --db --offline
```

Offline mode rejects missing entries, changed migrations, changed SQL, changed
fetch shapes, and unsupported cache versions.

`.dust_sql/` sits beside `pubspec.yaml` and is **committed**. It is a build
input rather than a build artifact: it is what lets a checkout with no database
validate its SQL, so `dust clean` leaves it alone and CI reads it directly.
Regenerate it with an online `dust db build` whenever migrations or query text
change, and commit the result in the same change.

Each entry records the driver it was described against, so a cache written for
one dialect is rejected against another rather than silently accepted.

## Transactions

Return `Ok` to commit and `Err` to roll back:

```dart
final result = await database.connection.transaction((tx) async {
  return UserDao(tx).createUser('ada@example.com', 'Ada');
});
```

Thrown exceptions also roll back and return a `SqlxDriverError`. Nested SQLite
transactions use savepoints, so an inner rollback does not undo outer work that
later commits.

```dart
await database.connection.transaction((tx) async {
  await UserDao(tx).createUser('ada@example.com', 'Ada');

  final nested = await tx.transaction<Unit>((nestedTx) async {
    await UserDao(nestedTx).createUser('bad@example.com', 'Bad');
    return Err<Unit, SqlxError>(SqlxError.driver('skip nested work'));
  });

  if (nested.isErr) {
    await UserDao(tx).createUser('grace@example.com', 'Grace');
  }
  return const Ok<Unit, SqlxError>(unit);
});
```

Transaction executors are scope-bound. Do not store `tx` and use it after the
callback returns; operations on a closed transaction return `Err(SqlxError)`.

For SQLite, use the supplied `tx` (including `UserDao(tx)`) throughout the
callback. Root driver/facade operations and close fail immediately while that
connection is owned; there is no implicit queue or retry. An active executor
also cannot escape its callback's async context. Await nested transactions:
their parent and parallel sibling operations are rejected until they finish,
and the parent waits for already-started children before commit/rollback.
Other connections remain independent. Failed rollback/savepoint cleanup blocks
further queries until close and reopen. Direct native-handle access bypasses
these guards. See the
[SQLite ownership example](../../packages/dust_db_sqlite3/example/transaction_ownership.dart).

## Column Aliases

The database infers whether a column can be null, and inference is sometimes
wrong in ways no schema can express: a `LEFT JOIN` makes a `NOT NULL` column
nullable in its result, and `max(x)` over an empty set is `NULL` while
`count(*)` is not.

Say so in the alias, as SQLx does:

```sql
SELECT o.total  as "total?",   -- nullable, whatever the schema says
       max(o.id) as "id!"      -- not null, whatever inference says
FROM orders o LEFT JOIN payments p ON p.order_id = o.id
```

The marker is part of the alias, so it reaches Dust as part of the column name
and is removed before the column is matched to a row field. A row class still
spells the column `total`, and the generated decoder still reads
`row.read<int>('total')`.

Only a trailing `!` or `?` is a marker. A name that merely contains one is left
alone.

## Type and Nullability Checks

`dust db build` compares each described column against the field reading it and
reports what disagrees. Both findings are **warnings**, not errors.

A column whose type no Dart field can hold:

```
warning: SQLx query `OrdersRepo.byItem` reads column `item` of type `TEXT`
         into `int`, which cannot hold it
```

A nullable column read into a non-nullable field:

```
warning: SQLx query `OrdersRepo.byItem` reads nullable column `total` into
         non-nullable `int`. Make the field nullable, or write `as "total!"`
         if the database is wrong about it
```

### Accepted type pairs

Anything not listed is accepted rather than reported, so a converter type, an
enum read through `tryFrom`, or a type this table has not learned costs nothing.

| Dart | SQLite | PostgreSQL |
| :--- | :--- | :--- |
| `int` | `INTEGER`, `NUMERIC`, `BOOLEAN` | `INT2`, `INT4`, `INT8`, `OID` |
| `double` | `REAL`, `NUMERIC`, `INTEGER` | `FLOAT4`, `FLOAT8`, `NUMERIC` |
| `num` | `INTEGER`, `REAL`, `NUMERIC` | the `int` and `double` rows |
| `bool` | `INTEGER`, `BOOLEAN`, `NUMERIC` | `BOOL` |
| `String` | `TEXT` | `TEXT`, `VARCHAR`, `CHAR`, `UUID`, `JSON`, `JSONB` |
| `DateTime` | `TEXT`, `DATETIME` | `TIMESTAMP`, `TIMESTAMPTZ`, `DATE` |
| `List<T>` | not supported | the matching PostgreSQL array type |

SQLite's rows are wide because it has type affinity rather than types: a column
declared `NUMERIC` holds whatever was written to it, and it has no boolean or
date type at all, so an integer reads into `bool` and text into `DateTime`.
PostgreSQL has real types, so its rows are narrow.

`numeric` is accepted into `double` provisionally. It is arbitrary-precision and
Dart has no counterpart, so the mapping is not settled; reading it as a `double`
loses precision.

### Nullability is checked per dialect

PostgreSQL describes nullability accurately and is checked. **SQLite is not**:
it describes a `PRIMARY KEY` column as nullable, which would warn about correct
code — `fixtures/server_app` produced five such warnings, every one wrong. The
column-alias overrides work on either dialect, so `as "total?"` still marks a
column nullable where SQLite cannot say so itself.

## Set Membership

An `IN` list does not need dynamic SQL. Bind the list itself and read it back
with `json_each`:

```dart
final orders = await queryAs<Order>(
  'SELECT id, item FROM orders WHERE id IN (SELECT value FROM json_each(?))',
  [ids],
).fetchAll(database.connection);
```

One placeholder, one bound value, and the SQL is constant — so `dust db build`
validates it like any other query. SQLite has no array type, so the driver binds
a `List` as JSON text; callers do not call `jsonEncode`. An empty list selects
no rows.

A `Uint8List` is bound as a BLOB rather than encoded, since that is what it is.

## Unchecked SQL

Migrations, `EXPLAIN`, one-off administrative work — the cases build-time
validation cannot reach. `unsafe` lives on the database facade:

```dart
final columns = await database.unsafe.fetch(
  'PRAGMA table_info(users)',
  const [],
);
```

It offers `fetch` for untyped rows, `fetchAs<T>` with an explicit mapper, and
`execute`. The mapper is passed by hand on purpose: generated terminals exist
only for validated queries, so the checked path stays the easy one.

`unsafe` is **not** on `Executor`. A request handler is handed an executor, and
no cast takes an executor to a `DatabaseClient`, so a handler cannot reach
unchecked SQL at all.

Each use warns, so it reads as a deliberate line in a diff rather than
disappearing into a file:

```
warning: unchecked SQL bypasses build-time validation
  --> lib/src/shared/db/database.dart:35:12
```

Silence one call with a marker comment on it or on the line above:

```dart
// dust:allow-unsafe-sql
await database.unsafe.execute('VACUUM', const []);
```

Two lines away does not count — one marker covers one call, never a file. The
warning is reported for the libraries Dust scans, so a helper in a file with no
Dust annotations is not seen.

Most reasons to reach for it have a checked answer:

| Instead of building SQL | Write |
| :--- | :--- |
| `IN (?, ?, ?)` | one bound list over `json_each` — see [Set Membership](#set-membership) |
| optional filters | a `switch` over the supplied combination, each branch a constant query |
| a dynamic `ORDER BY` | a `switch` over an enum |

A sort column is an identifier and no dialect binds one, so string building is
the only mechanism available — which is exactly why it is the classic injection
site, and why it belongs in a `switch`:

```dart
final orders = await switch (sort) {
  OrderSort.newest => queryAs<Order>(_byPlacedAt, [accountId]),
  OrderSort.item => queryAs<Order>(_byItem, [accountId]),
}
    .fetchAll(database.connection);
```

Every branch is a constant string `describe` accepted, and the switch is
exhaustive.

For advanced SQLite-specific operations, access the native database explicitly:

```dart
final sqlite = (database.connection as Sqlite3Executor).database;
final version = sqlite.select('SELECT sqlite_version()').single[0];
```

> [!TIP]
> Prefer `database.connection` plus generated DAOs for product queries. Keep
> `unsafe` and native access small because neither path receives Dust's
> build-time SQL validation.

## Example

The [shopping app database](../../examples/shopping_app/lib/core/db/shopping_cache_database.dart)
demonstrates generated DAOs, migrations, typed rows, JSON columns, converters,
transactions, and cache-backed repository behavior.
