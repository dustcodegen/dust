# Primary constructors: development status

Work in progress under [#76](https://github.com/dustcodegen/dust/issues/76),
not a published Dust feature. Do not migrate production models on the strength
of grammar acceptance alone. Versions and runtime SDK floors are unchanged.

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

These are scoped tests, not proof of every primary-constructor form. Inferred
declaring-parameter types, generic/inherited field combinations and full
acceptance checks still need complete validation.

Remaining gaps in this draft:

- [Shorthand callback declaring parameters (#92)](https://github.com/dustcodegen/dust/issues/92),
  such as `final bool predicate(String value)`, currently lose the callable
  signature and can produce invalid `copyWith` code. The explicit
  `final bool Function(String) predicate` form compiled in the scoped
  latest-Dart probe; it does not need a new handwritten API. Source-preserving
  effective-function parser facts need review before fixing the shorthand.
- [Generic class generation (#93)](https://github.com/dustcodegen/dust/issues/93)
  now has scoped executable typed-copy checks for traditional and primary
  classes using `with _$Box<T>`. These cover bounds, nullable arguments,
  nested helpers, equality/debug output and typed imported replacements.
  Core-name/local collisions have separate traditional/primary executable
  probes. The [approved design](../design/generic-models.md) still requires
  generated-member collisions (#103), generic validation helpers and integration
  capability checks. Passing the copy cases does not establish all generic derives.

For these cases, a successful `dust build` does not establish that the generated
Dart compiles. Do not use the draft feature as production-readiness evidence.

Keep verified traditional declarations for production until the issue's full
acceptance gates pass. Named concise `new` constructors and dot shorthands remain separate
unsupported features and fail clearly.
