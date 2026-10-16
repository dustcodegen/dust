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
- `fixtures/primary_flutter_consumer`: primary route widgets with typed path/query
  fields and super keys; primary state and ViewModel declarations; generated
  route parsing and a scope-driven widget rebuild after a state update.

The SQLite statements in this consumer are unchecked runtime SQL. Generated
`FromRow` mapping is exercised, but these tests do not prove build-time SQL
validation. Checked statements still use Dust's sqlx-backed [DB mode](db.md).

These are scoped tests, not proof of every primary-constructor form. Inferred
declaring-parameter types, generic/inherited field combinations and full
acceptance checks still need complete validation.
Keep traditional declarations for production until the issue's full acceptance
gates pass. Named concise `new` constructors and dot shorthands remain separate
unsupported features and fail clearly.
