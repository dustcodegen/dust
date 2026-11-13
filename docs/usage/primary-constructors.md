# Primary constructors

Dust supports class-header primary constructors from 0.3.0 onward. Traditional
constructors remain supported; choose either form for the documented derives.

## Set the language version

Primary constructors require a Dart 3.13 or newer SDK and a package language
version of at least 3.13. Set the lower SDK bound in your application's
`pubspec.yaml`:

```yaml
environment:
  sdk: ">=3.13.0 <4.0.0"
```

This enables the syntax in your application; it does not raise Dust's runtime
package SDK floors. Add `dust_dart` and the CLI using the
[installation guide](../../README.md#installation).

## Positional fields

The class header declares the constructor. `final` and `var` parameters declare
fields; ordinary parameters without either keyword do not.

In `user.dart`:

```dart
import 'package:dust_dart/derive.dart';

part 'user.g.dart';

@Derive([Eq(), CopyWith()])
class const User(final String name) with _$User;
```

The equivalent traditional declaration uses the same derives and generated API:

```dart
@Derive([Eq(), CopyWith()])
class TraditionalUser with _$TraditionalUser {
  const TraditionalUser(this.name);

  final String name;
}
```

Generate and check from the application package root:

```bash
dust build
dust check
```

Constructor calls and generated member calls keep their usual form:

```dart
final renamed = const User('Ada').copyWith(name: 'Grace');
assert(renamed == const User('Grace'));
```

No `this;` body is needed for this form. Add a `this` part when you need
initializers, a constructor body, constructor metadata or documentation. Some
strict documentation lints require a public constructor to have its own docs;
that is not a Dust requirement. See
[Dart's primary-constructor guide](https://dart.dev/language/primary-constructors).

## Named fields and defaults

Use braces for named parameters. In `account.dart`:

```dart
import 'package:dust_dart/serde.dart';

part 'account.g.dart';

@Derive([Eq(), CopyWith(), Serialize(), Deserialize()])
class const Account({
  required final String name,
  final String? note,
  @SerDe(defaultValue: 0) final int visits = 0,
}) with _$Account {
  factory Account.fromJson(Map<String, Object?> json) => _$AccountFromJson(json);
}
```

A constructor default applies to Dart calls. `@SerDe(defaultValue: 0)` separately
defines the value used for an absent JSON key; the constructor default alone
would not make that key optional during decoding.

```dart
const account = Account(name: 'Ada');
final saved = account.copyWith(note: 'saved', visits: 2);
final cleared = saved.copyWith(note: null);
final decoded = Account.fromJson({'name': 'Ada'});
assert(decoded == account);
assert(cleared.note == null);
```

The [JSON guide](serde.md) documents rename rules, wire defaults and codecs;
the [data-class guide](derive.md) documents equality and copy behavior.

## Named constructors

Put the constructor name after the class name. The generated copy uses that
constructor and retains omitted values:

```dart
@Derive([CopyWith()])
class Counter.named(var int value, [final String label = 'counter'])
    with _$Counter;
```

Call it by its declared name:

```dart
final updated = Counter.named(1).copyWith(value: 2);
assert(updated.value == 2 && updated.label == 'counter');
```

This is a class-header constructor. Named concise `new` declarations are a
separate unsupported feature, as described below.

## Row models

Primary constructors work with `FromRow()` too. In `account_row.dart`:

```dart
import 'package:dust_dart/db.dart';

part 'account_row.g.dart';

@Derive([FromRow()])
class const AccountRow(final String name, final String? note);
```

`FromRow()` generates a conversion function, a public row deserializer and typed
query terminals, so this row-only class needs no generated mixin. Keep the row
model in its normal-derive library and import it from the database/DAO library.
After configuring the database and migrations, generate and verify both modes:

```bash
dust build
dust db build
dust check
dust check --db
```

The [Database row-mapping guide](db.md#row-mapping) explains constructor
requirements, SQL column names, defaults and the generated query API.

## Executable checks

The local [SDK gate](sdk-validation.md) stages fresh independent consumers:

- `fixtures/primary_sdk_consumer`: typed named/positional fields, named and const
  headers, defaults, JSON rename/default behavior, validation, equality and
  nullable copyWith omission versus explicit null. Private named fields use
  public constructor/copyWith labels while retaining private field access;
  JSON aliases and SQLx row-column names stay independent. Nested private copies,
  generated-local collisions and traditional private initializing formals are
  exercised too. Conflicting copyWith labels such as `_name` and `name` fail
  clearly instead of producing duplicate arguments.
  Primary enums retain their fields and generated variant aliases; trivial
  mixin-class primaries and extension representations coexist with models.
  HTTP client tests serialize requests and decode responses without a network
  call. Real in-memory SQLite tests execute generated row queries against `db`
  and `tx` in both sync and worker modes, including transaction rollback.
  Checked application-library functions and generated DAO queries exercise all
  five terminals against the same private primary row model and migrations.
  The SDK gate runs online SQLx validation and an offline metadata check before
  analysis and execution; it ignores any host `DUST_DATABASE_URL`.
- `fixtures/primary_flutter_consumer`: primary route widgets with typed path/query
  fields and super keys; primary state and ViewModel declarations; generated
  route parsing and a scope-driven widget rebuild after a state update.

Statements in `test/sqlite_runtime_test.dart` remain unchecked runtime SQL.
The separate `lib/checked_queries.dart` functions and `lib/database.dart` DAO
are validated through Dust's sqlx-backed [DB mode](db.md). This checks static
SQL, bind shapes and required row columns, not every possible SQL type or
runtime value; the [online/offline limits](db-query-parity.md#what-rust-sqlx-checks)
still apply.

Imported primary row classes use shared parsed workspace facts during DAO
validation and emission, not a source-line guess at the class header.

These executable checks define the supported source boundary. They cover typed
declaring parameters, generic and inherited fields, constructor defaults,
private names, callbacks and the Dust features listed above. They do not claim
support for every Dart declaration form; syntax outside this documented surface
must continue to fail clearly instead of generating incomplete code.

Separate unsupported forms and limits:

- Named mixin applications (`class Alias = Base with Mix;`) retain declaration
  identity for annotation ownership, but Dust generation for those classes
  remains unsupported. Their implicit forwarding constructors are not ordinary
  primary or body constructors. Use an ordinary class with explicit constructors
  for generated models; [#155](https://github.com/dustcodegen/dust/issues/155)
  tracks the namespace and explicit-diagnostic boundary.
- [Shorthand callback declaring parameters (#92)](https://github.com/dustcodegen/dust/issues/92),
  such as `final bool predicate(String value)`, now retain a separate effective
  function type through copy generation. Explicit `Function(...)` syntax is
  still recommended by Dart's style lint. Scoped minimum/latest executable
  probes cover nullable returns/functions, generic/nested/named invocation and
  callback retain/replace/null-clear; inference and defaults inside signatures
  fail clearly. See the
  [executable fixtures](../../fixtures/primary_callback_formals/README.md).
- [Generic class generation (#93)](https://github.com/dustcodegen/dust/issues/93)
  is supported for the capabilities exercised by the traditional and primary
  consumers using `with _$Box<T>`. These cover bounds, nullable arguments,
  nested helpers, equality/debug output, validation, SerDe, inheritance, sealed
  models and typed imported replacements. Dust rejects names that collide with
  generated members such as `copyWith`; this is not a claim that every generic
  derive combination is supported.

Named concise `new` constructors and dot shorthands remain separate unsupported
features and fail clearly. Named mixin applications remain unsupported generation
targets.
