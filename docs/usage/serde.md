# JSON Serialization

Dust generates typed JSON encoding and decoding for Dart classes and enums.

## Rust Serde Inspiration

Dust's JSON API is inspired by [Rust Serde](https://serde.rs/derive.html).
Both systems let a type opt into separate serialization and deserialization,
then customize declarations, variants, and fields with
[attributes](https://serde.rs/attributes.html).

Dust adapts familiar concepts such as rename rules, aliases, defaults, skipped
fields, strict keys, custom codecs, and
[tagged enum representations](https://serde.rs/enum-representations.html) to
Dart annotations and generated `.g.dart` files.

## Add the Package

Install the Dust CLI from the [root guide](../../README.md#installation), then
add the Dart runtime package:

```bash
dart pub add dust_dart
```

> [!TIP]
> The focused `package:dust_dart/serde.dart` import also exports `@Derive` and
> the core derive traits, so one import is enough for data and JSON traits.

## Quick Start

Add `Serialize()` and `Deserialize()` to the traits needed by the model:

```dart
import 'package:dust_dart/serde.dart';

part 'user_profile.g.dart';

@Derive([Serialize(), Deserialize()])
@SerDe(renameAll: SerDeRename.snakeCase)
class UserProfile with _$UserProfile {
  const UserProfile({required this.id, this.displayName});

  factory UserProfile.fromJson(Map<String, Object?> json) =>
      _$UserProfileFromJson(json);

  final String id;
  final String? displayName;
}
```

Generate the part file:

```bash
dust build
```

Use the generated APIs normally:

```dart
final profile = UserProfile.fromJson({
  'id': 'u1',
  'display_name': 'Jane',
});

final json = profile.serialize();
final dartJson = profile.toJson(); // Dart ecosystem mirror.
```

`Serialize()` generates `serialize()` as the Dust source API and `toJson()` as
the Dart ecosystem mirror. `Deserialize()` generates the private
`_$TypeDeserialize(...)` source helper and `_$TypeFromJson(...)` mirror helper;
a class must expose a forwarding `fromJson` factory as shown above. The two
traits can be used independently.

Dust also generates reusable directional objects:

```dart
const serializer = $UserProfileSerializer();
final json = serializer.serialize(profile);

const deserializer = $UserProfileDeserializer();
final profileAgain = deserializer.deserialize(json);
```

These implement `Serializer<UserProfile, Map<String, Object?>>` and
`Deserializer<UserProfile, Map<String, Object?>>`. They are useful when a
framework, registry, or helper wants conversion behavior as a value. Those
contracts require only the Dust source verbs; the `serde.dart` import provides
`toJson` and `fromJson` extension mirrors for Dart ecosystem naming.

> [!IMPORTANT]
> Generation requires the matching `part` directive and generated mixin.
> Deserialization also requires the forwarding `fromJson` factory shown above.

## Configuration

Apply `@SerDe` to a class, enum, enum variant, or field.

### Declaration and Variant Options

| Option | Behavior |
| :--- | :--- |
| `rename` | Sets one explicit JSON name. |
| `renameAll` | Applies a naming strategy to fields or enum variants. |
| `disallowUnrecognizedKeys` | Rejects input keys that do not map to a deserialized field or alias. |
| `tag` | Sets the discriminator key for sealed variants. |
| `content` | Adds a payload key for adjacent tagging. |
| `untagged` | Tries sealed variants in declaration order without a discriminator. |

### Field Options

| Option | Behavior |
| :--- | :--- |
| `rename` | Sets the primary JSON key. |
| `aliases` | Accepts additional keys during decoding. |
| `defaultValue` | Supplies a value when the input key is absent. |
| `skip` | Omits the field in both directions. |
| `skipSerializing` | Omits the field from `serialize()` and `toJson()`. |
| `skipDeserializing` | Omits the field from `_$TypeDeserialize(...)` and `fromJson()`. |
| `using` | Uses a `SerDeCodec` for the field. |

> [!IMPORTANT]
> Fields skipped during deserialization must have a `defaultValue` so Dust can
> still call the model constructor. In strict-key mode, skipped input keys are
> rejected rather than silently ignored.

## Rename Strategies

| Strategy | `createdAt` becomes |
| :--- | :--- |
| `lowerCase` | `createdat` |
| `upperCase` | `CREATEDAT` |
| `pascalCase` | `CreatedAt` |
| `camelCase` | `createdAt` |
| `snakeCase` | `created_at` |
| `screamingSnakeCase` | `CREATED_AT` |
| `kebabCase` | `created-at` |
| `screamingKebabCase` | `CREATED-AT` |

## Supported Types

Dust provides built-in mapping for:

- `String`, `int`, `bool`, `double`, `num`, `Object`, and `dynamic`
- `DateTime`, `Uri`, and `BigInt`
- nullable values
- `List<T>`, `Set<T>`, and `Map<String, T>`
- Dust SerDe classes and enums
- models with compatible `toJson()` or `fromJson(Map<String, Object?>)` APIs
- fields with a custom `SerDeCodec`

Records, function types, and generic named models such as `Page<User>` require
a custom codec. Dust validates JSON capability for types declared in the
current workspace; external package types are checked later by the Dart
analyzer.

## Generic Models (Development Branch)

Ordinary generic models retain their chosen arguments and bounds in generated
helpers, mixins and directional codec classes. Keep the mixin applied to the
model's parameters, and forward the factory to a typed helper:

```dart
@Derive([Serialize(), Deserialize()])
class Label<T extends num> with _$Label<T> {
  const Label(this.text);

  factory Label.fromJson(Map<String, Object?> json) => _$LabelFromJson<T>(json);

  final String text;
}

final Label<int> label = Label<int>.fromJson({'text': 'ready'});
const serializer = $LabelSerializer<int>();
const deserializer = $LabelDeserializer<int>();
final Label<int> decoded = deserializer.deserialize(serializer.serialize(label));
```

The concrete `String` field needs no conversion for `T`. A field using its class parameter
(for example `T` or `List<T?>`) requires explicit `@SerDe(using: ...)` mapping.
Skipped directions do not need parameter conversion; deserialization defaults
still apply. Bounds do not imply a JSON conversion. A parameter named `DateTime`, `List` or
another SDK type is still a parameter, not that SDK type; Dust reports the field
instead of guessing its conversion. No generic decoder-callback API is added.

This is unreleased, scoped support under
[#128](https://github.com/dustcodegen/dust/issues/128), not general generic-model
readiness. Same-library applied inheritance under
[#133](https://github.com/dustcodegen/dust/issues/133) now retains actual inherited
types: `Concrete extends ScalarBase<int>` generates integer JSON conversions and
integer copy parameters, not the parent's `T`. Inherited field annotations remain
effective. The [complete concrete example](../../fixtures/generic_inheritance/concrete_traditional.dart.snapshot)
supports normal usage:

```dart
final count = Concrete.fromJson({'amount': 3});
final updated = count.copyWith(amount: 7);
assert(updated.toJson()['amount'] == 7);
assert(updated.validate().isValid);
```

The [inheritance fixtures](../../fixtures/generic_inheritance) also exercise named
super-parameters, nested nullable collections, dependent/reordered bounds,
callback overrides and primary constructors. Only proven local class parents
are substituted; imported parents and alias expansion are not inferred. Raw or
wrong-arity applications and substitutions inside opaque function/record types
fail clearly. An explicit effective child field can override an opaque parent
field without rewriting it. A child field that still uses its own parameter
continues to require explicit JSON mapping; bounds are not decoder functions.

Generic sealed redirects are implemented on the development branch under
[#134](https://github.com/dustcodegen/dust/issues/134), with acceptance still in
progress. Their declared or omitted targets retain applied arguments, bounds,
named constructors and `.new`; identifiers never contain `<T>`.
For example, the [complete declared family](../../fixtures/generic_sealed/declared_traditional.dart.snapshot)
uses `Event<A, B extends List<A>>` and redirects `Event.done` to
`Done<B, A>.create`:

```dart
const event = Event<int, List<int>>.done('ready');
final json = event.serialize(); // {'label': 'ready', 'type': 'done'}
final Event<int, List<int>> restored = Event<int, List<int>>.fromJson(json);
final codecResult = const $EventDeserializer<int, List<int>>().deserialize(json);
```

Sealed decoding honors the redirect's constructor even when the ordinary leaf
codec selects a different first constructible constructor. Tagged, adjacent and
untagged representations retain their owning types. Unknown generic subtypes
outside the factory protocol throw `ArgumentError`; Dart cannot prove every
dependent generic subtype pattern exhaustive. Manual JSON methods remain the
manual variant's responsibility.

Omitted generic targets must carry a complete permutation of the owner's bare
parameters. Dust copies those source binders and bounds, rather than inventing
names for concrete/nested applications. Unproven imported ownership, invalid
arity and opaque substitutions fail before replacing generated output/cache.
Factory-parameter `@SerDe` metadata supplies generated field codecs, aliases and
wire defaults. Redirecting factories cannot declare explicit Dart defaults;
nullable optional parameters still default to `null`. When source binders capture
runtime JSON names, a generated public factory uses an allocated public payload
typedef such as `$CaptureDoneJson`, not private aliases in its public signature.

All constructor forms are **not** production-ready: required nullable named and
optional positional parameters need the separate
[#138](https://github.com/dustcodegen/dust/issues/138) fix. Do not use those omitted
variant forms yet. See the [generic-model design](../design/generic-models.md),
[sealed fixtures](../../fixtures/generic_sealed) and
[ordinary generic JSON fixtures](../../fixtures/generic_serde).

## Enums

Enums can derive both directions and use declaration or variant renames:

```dart
@Derive([Serialize(), Deserialize()])
@SerDe(renameAll: SerDeRename.kebabCase)
enum UserRole {
  admin,
  moderator,
  @SerDe(rename: 'root')
  superUser,
  @SerDe(skip: true)
  legacy,
}
```

> [!NOTE]
> A skipped enum value remains valid Dart, but generated JSON helpers reject it
> for both encoding and decoding.

## Sealed Classes

Redirecting factories define sealed variants. `tag` creates an internal
discriminator, `tag` plus `content` creates adjacent tagging, and `untagged`
tries each variant in declaration order.

```dart
@Derive([Serialize(), Deserialize()])
@SerDe(tag: 'type', renameAll: SerDeRename.snakeCase)
sealed class PaymentEvent with _$PaymentEvent {
  const PaymentEvent();

  factory PaymentEvent.fromJson(Map<String, Object?> json) =>
      _$PaymentEventFromJson(json);

  @SerDe(rename: 'payment_success')
  factory PaymentEvent.success({
    required String id,
    required int cents,
  }) = PaymentSuccess;

  factory PaymentEvent.failed({
    required String id,
    required String reason,
  }) = PaymentFailed;
}
```

If a redirect target is not declared in source, Dust generates a final concrete
class in the `.g.dart` file. Define the target class yourself when you need to
control its implementation; it must still satisfy the redirecting factory and
Serde requirements.

## Custom Codecs

Use `SerDeCodec<DartT, JsonT>` when the JSON representation differs from the
Dart type. `JsonT` is the exact intermediate JSON shape used by the field:

```dart
final class UnixEpochCodec implements SerDeCodec<DateTime, int> {
  const UnixEpochCodec();

  @override
  int serialize(DateTime value) => value.millisecondsSinceEpoch;

  @override
  DateTime deserialize(int value) =>
      DateTime.fromMillisecondsSinceEpoch(value, isUtc: true);
}

const unixEpochCodec = UnixEpochCodec();

@SerDe(using: unixEpochCodec)
final DateTime createdAt;
```

> [!TIP]
> Dust handles field nullability outside the codec. The codec converts only the
> non-null value. The `serde.dart` import provides `toJson` and `fromJson`
> extension mirrors, so custom codecs usually implement only `serialize` and
> `deserialize`.

> [!TIP]
> Dust emits typed codec calls. If `@SerDe(using: ...)` points a `DateTime`
> field at a codec for another Dart type, `dart analyze` reports the mismatch
> in the generated file. If the incoming JSON has the wrong shape for the codec,
> Dust wraps the failure with the field key.

## Examples

- [Class options and strict keys](../../examples/product_showcase/lib/models/json_serde_options.dart)
- [Enums](../../examples/product_showcase/lib/models/json_enum_bundle.dart)
- [Sealed classes](../../examples/product_showcase/lib/models/json_payment_event.dart)
- [Custom codecs](../../examples/product_showcase/lib/models/json_codec_bundle.dart)
- [Serde tests](../../examples/product_showcase/test/generated_serde_scalars_test.dart)
