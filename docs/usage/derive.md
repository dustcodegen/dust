# Data Classes

Dust generates `toString`, value equality, and typed `copyWith` APIs for Dart
classes.

## Rust Inspiration

Dust's `@Derive([...])` follows the idea behind
[Rust's `#[derive(...)]`](https://doc.rust-lang.org/book/appendix-03-derivable-traits.html):
each type opts into the generated traits it needs. Rust expands derives through
the compiler and macros; Dust writes a Dart `.g.dart` part and exposes the
generated behavior through a mixin.

## Add the Package

Install the Dust CLI from the [root guide](../../README.md#installation), then
add the Dart runtime package:

```bash
dart pub add dust_dart
```

## Quick Start

Choose the traits you want with `@Derive`:

```dart
import 'package:dust_dart/derive.dart';

part 'product.g.dart';

@Derive([ToString(), Eq(), CopyWith()])
class Product with _$Product {
  const Product({
    required this.id,
    required this.name,
    required this.priceCents,
  });

  final String id;
  final String name;
  final int priceCents;
}
```

Generate the part file from your package root:

```bash
dust build
```

For a library that fails validation, Dust leaves its last successful generated
output and cache entry unchanged. Fix the diagnostic and run `dust build` again;
deleting the cache or editing generated files is not required. Other valid
libraries in the same build can still update normally.

You can now use the generated members normally:

```dart
const product = Product(id: 'p1', name: 'Desk', priceCents: 12000);

print(product);
final renamed = product.copyWith(name: 'Standing desk');
final sameValue = product ==
    const Product(id: 'p1', name: 'Desk', priceCents: 12000);
```

## Traits

| Trait | Generated API |
| :--- | :--- |
| `ToString()` | A `toString()` implementation containing the class name, field names, and values. |
| `Eq()` | Value-based `operator ==` and a matching `hashCode`. |
| `CopyWith()` | A typed, callable `copyWith` API for replacing selected fields. |

> [!TIP]
> Derive only the traits the class needs. Each trait adds only its corresponding
> generated API.

## Class Requirements

> [!IMPORTANT]
> Every derived class needs a `part 'filename.g.dart';` directive and the
> generated `_$ClassName` mixin shown in the Quick Start.

- A class using `CopyWith()` must be concrete and have a constructor that
  accepts every field.
- Existing superclasses and mixins are supported; place the generated mixin in
  the normal mixin list.
- Dust does not generate derive members for `mixin class` targets.

Unannotated `mixin class` helpers can live beside derived classes and participate
in their mixin lists. The helper itself is not a derive target: adding a
Dust-owned derive trait to it still fails with an unsupported-target diagnostic.
The [Product example](../../examples/product_showcase/lib/models/product.dart)
uses a handwritten mixin-class label getter alongside generated equality and
copyWith; replacing a model field preserves the inherited helper behavior.

Fields are usually `final` for data-class usage, but Dust does not require
immutability.

## Value Equality

`Eq()` compares the runtime type and every field. Collections are compared by
contents:

- `List` and `Iterable` compare elements in order.
- `Map` compares keys and values by content.
- `Set` uses unordered deep equality.
- Generated `hashCode` uses the same collection rules.

Nested model fields use their own equality implementation.

## CopyWith Behavior

`CopyWith()` keeps omitted fields and replaces only the arguments you pass:

```dart
final renamed = product.copyWith(name: 'Standing desk');
```

Nullable fields can be cleared explicitly:

```dart
final cleared = note.copyWith(note: null);
```

> [!NOTE]
> Copying is shallow. Dust keeps existing object and collection references when
> you do not replace them, and it stores replacement values without cloning
> them.

When a field is another model that also derives `CopyWith()`, Dust generates a
chained helper:

```dart
final moved = profile.copyWith.address(city: 'London');
```

### Generic Models

Dust preserves chosen type arguments through generated mixins and copy helpers,
as specified in [#93](https://github.com/dustcodegen/dust/issues/93):

```dart
@Derive([CopyWith(), ToString(), Eq()])
class Box<T> with _$Box<T> {
  const Box(this.value);

  final T value;
}

final Box<int> updated = const Box<int>(1).copyWith(value: 2);
final Box<String?> cleared = const Box<String?>('before').copyWith(value: null);
```

Pass every class parameter to the mixin in declaration order. Dust rejects
missing or substituted arguments rather than silently erasing types.
Omission keeps the field; explicit null clears it only when the instantiated
field type accepts null. For non-nullable fields, null keeps the existing value.
Copies stay shallow. Same-library nested helpers retain their type arguments;
imported models support typed replacement, not private chained helpers.

Traditional and primary-constructor consumers exercise these cases locally.
Core-name/local collision cases have scoped tests, but names matching generated
members such as `copyWith` still need
[#103](https://github.com/dustcodegen/dust/issues/103). Generic validation helpers
and other plugin capabilities remain under the [generic model design](../design/generic-models.md).

### Explicit Generic Callbacks

A non-generic model can contain a callback with its own type parameters:

```dart
@Derive([CopyWith()])
class CallbackModel with _$CallbackModel {
  const CallbackModel({required this.transform, this.optionalTransform});

  final T Function<T>(T) transform;
  final T Function<T>(T)? optionalTransform;
}
```

Write the full function type on the field. The callback's `T` belongs to its
function signature; it does not make `CallbackModel` a generic class.

```dart
T identity<T>(T value) => value;

final model = CallbackModel(transform: identity, optionalTransform: identity);
final copied = model.copyWith();
final int result = copied.transform<int>(42);
final cleared = model.copyWith(optionalTransform: null);
```

Omission retains the callback. Passing `null` clears a nullable callback, while
the required callback keeps the existing non-nullable null-as-omission behavior.
This example does not establish generic class generation (#93) or shorthand
primary callback-formal support (#92).

## Examples

- [Product showcase models](../../examples/product_showcase/lib/models)
- [Generated derive tests](../../examples/product_showcase/test/generated_derive_models_test.dart)
- [Generated copyWith tests](../../examples/product_showcase/test/generated_copywith_test.dart)
- [Explicit callback model](../../examples/product_showcase/lib/models/callback_model.dart)
- [Callback copy tests](../../examples/product_showcase/test/models/callback_model_test.dart)
- [Draft generic models](../../examples/product_showcase/lib/models/generic_models.dart)
- [Generic copy tests](../../examples/product_showcase/test/models/generic_models_test.dart)
