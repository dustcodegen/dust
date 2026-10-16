import 'dart:async';

import 'package:dust_dart/derive.dart';

part 'generic_models.g.dart';

/// A nullable alias whose nullability is not a written question mark at use sites.
typedef Maybe<T> = T?;

/// A typed generic value with normal instance derives.
@Derive([CopyWith(), ToString(), Eq()])
class GenericBox<T> with _$GenericBox<T> {
  /// Creates a box.
  const GenericBox(this.value);

  /// The boxed value, which may be nullable after instantiation.
  final T value;
}

/// A bounded value using a named constructor.
@Derive([CopyWith()])
class BoundedBox<T extends num> with _$BoundedBox<T> {
  /// Creates a bounded box.
  const BoundedBox.named(this.value);

  /// The numeric value.
  final T value;
}

/// Different outer shapes with runtime-dependent nullability.
@Derive([CopyWith()])
class GenericValues<T> with _$GenericValues<T> {
  /// Creates values with a non-nullable outer list and an optional alias.
  const GenericValues(
      {required this.values, required this.pending, this.maybe});

  /// A non-nullable list containing nullable elements.
  final List<T?> values;

  /// A future or immediate value; only some instantiations accept null.
  final FutureOr<T> pending;

  /// An alias that accepts null even though this use does not spell `?`.
  final Maybe<T> maybe;
}

/// Opaque callback/record source still refers to the class's own parameter.
@Derive([CopyWith()])
class GenericShapes<T> with _$GenericShapes<T> {
  /// Creates structured values.
  const GenericShapes({required this.row, required this.choose});

  /// A record containing the chosen type.
  final ({T item, int count}) row;

  /// A callback with its own independent type parameter.
  final T Function<U>(T, U) choose;
}

/// Nested helpers must forward the actual child type argument.
@Derive([CopyWith()])
class GenericEnvelope<T> with _$GenericEnvelope<T> {
  /// Creates an envelope.
  const GenericEnvelope({required this.child, this.optional});

  /// The required child.
  final GenericBox<T> child;

  /// A child whose nested helper is nullable.
  final GenericBox<T>? optional;
}

/// A binder that must not collide with the generated copy result parameter.
@Derive([CopyWith()])
class ResultNamed<$Res> with _$ResultNamed<$Res> {
  /// Creates a value.
  const ResultNamed(this.value);

  /// The value.
  final $Res value;
}

/// An unused class parameter still belongs in the copy's result type.
@Derive([CopyWith()])
class EmptyBox<T> with _$EmptyBox<T> {
  /// Creates an empty box.
  const EmptyBox();
}

/// Later parameters can depend on earlier class binders.
@Derive([CopyWith()])
class DependentBox<T, U extends List<T>> with _$DependentBox<T, U> {
  /// Creates a dependent box.
  const DependentBox(this.value, this.values);

  /// The selected value.
  final T value;

  /// A collection of the same element type.
  final U values;
}

/// Recursive bounds belong on declarations, not constructor use sites.
@Derive([CopyWith()])
class RecursiveBox<T extends Comparable<T>> with _$RecursiveBox<T> {
  /// Creates a recursively bounded box.
  const RecursiveBox(this.value);

  /// A value comparable with the same type.
  final T value;
}

/// Nested helpers must preserve compound actual arguments too.
@Derive([CopyWith()])
class GenericListEnvelope<T> with _$GenericListEnvelope<T> {
  /// Creates a list envelope.
  const GenericListEnvelope(this.child);

  /// A child containing a non-nullable list with nullable elements.
  final GenericBox<List<T?>> child;
}
