# Primary constructors

Dust v0.3.0 is scheduled to support class-header primary constructors under
[#76](https://github.com/dustcodegen/dust/issues/76). Until it is published,
build the CLI from this checkout and use the matching source packages together.
Versions and runtime SDK floors remain unchanged, and traditional declarations
remain fully supported.

Dart primary constructors require language version 3.13 or newer. The class
header declares the constructor; `final` or `var` parameters induce fields:

```dart
import 'package:dust_dart/derive.dart';

part 'user.g.dart';

@Derive([ToString()])
class const User(final String name) with _$User;

final user = User('Ada');
```

No `this;` body is needed for this form. Add a `this` part only when needed for
initializers, a constructor body, metadata or constructor documentation. Some
strict documentation lints require documenting a public constructor separately;
that is not a Dust requirement. Ordinary header parameters without `final` or
`var` do not create fields. See [Dart's primary-constructor guide](https://dart.dev/language/primary-constructors).

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

Imported primary row classes use the shared parsed workspace facts during DAO
validation and emission, not a source-line guess at the class header. Keep the
row model in its normal-derive library and import it from the DAO library:

```dart
import 'package:dust_dart/db.dart';

part 'account_row.g.dart';

@Derive([FromRow()])
class const AccountRow(final String name, final String? note);
```

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
  fail clearly. See the [approved contract](../design/function-formal-types.md)
  and [executable fixtures](../../fixtures/primary_callback_formals/README.md).
- [Generic class generation (#93)](https://github.com/dustcodegen/dust/issues/93)
  is supported for the capabilities exercised by the traditional and primary
  consumers using `with _$Box<T>`. These cover bounds, nullable arguments,
  nested helpers, equality/debug output, validation, SerDe, inheritance, sealed
  models and typed imported replacements. Names that collide with generated
  members such as `copyWith` remain tracked separately in
  [#103](https://github.com/dustcodegen/dust/issues/103); this is not a claim
  that every generic derive combination is supported.

Named concise `new` constructors and dot shorthands remain separate unsupported
features and fail clearly. Named mixin applications remain unsupported generation
targets. The published 0.2.1 release still requires traditional declarations;
primary-constructor projects must use a source-built CLI and matching source
packages until v0.3.0 is published.
