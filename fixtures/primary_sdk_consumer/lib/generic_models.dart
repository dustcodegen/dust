import 'dart:async';

import 'package:dust_dart/derive.dart';

part 'generic_models.g.dart';

/// A nullable alias without a written question mark at use sites.
typedef Maybe<T> = T?;

/// A primary model retaining its selected type through instance derives.
@Derive([CopyWith(), ToString(), Eq()])
class const GenericBox<T>(
  /// The boxed value.
  final T value,
) with _$GenericBox<T> {
  /// Creates a box.
  this;
}

/// A primary named constructor with a written bound.
@Derive([CopyWith()])
class const BoundedBox<T extends num>.named(
  /// The bounded value.
  final T value,
) with _$BoundedBox<T> {
  /// Creates a bounded box.
  this;
}

/// Container, FutureOr and alias nullability after instantiation.
@Derive([CopyWith()])
class const GenericValues<T>({
  /// A non-nullable outer list.
  required final List<T?> values,

  /// A future or value of the chosen type.
  required final FutureOr<T> pending,

  /// An optional nullable alias.
  final Maybe<T> maybe,
}) with _$GenericValues<T> {
  /// Creates structured values.
  this;
}

/// Record and callback source retains its lexical binders.
@Derive([CopyWith()])
class const GenericShapes<T>({
  /// A record containing the chosen type.
  required final ({T item, int count}) row,

  /// A callback with its own independent parameter.
  required final T Function<U>(T, U) choose,
}) with _$GenericShapes<T> {
  /// Creates callback and record values.
  this;
}

/// A primary model whose nested helpers retain actual arguments.
@Derive([CopyWith()])
class const GenericEnvelope<T>({
  /// The required child.
  required final GenericBox<T> child,

  /// An optional child.
  final GenericBox<T>? optional,
}) with _$GenericEnvelope<T> {
  /// Creates an envelope.
  this;
}

/// A class binder must not collide with the generated result binder.
@Derive([CopyWith()])
class const ResultNamed<$Res>(
  /// The chosen value.
  final $Res value,
) with _$ResultNamed<$Res> {
  /// Creates a value.
  this;
}

/// Phantom type arguments remain part of a copied value's type.
@Derive([CopyWith()])
class const EmptyBox<T>() with _$EmptyBox<T> {
  /// Creates an empty box.
  this;
}

/// Later binders can depend on earlier class parameters.
@Derive([CopyWith()])
class const DependentBox<T, U extends List<T>>(
  /// The chosen value.
  final T value,

  /// A collection of that value type.
  final U values,
) with _$DependentBox<T, U> {
  /// Creates a dependent box.
  this;
}

/// Recursive bounds belong to declarations rather than constructor use sites.
@Derive([CopyWith()])
class const RecursiveBox<T extends Comparable<T>>(
  /// The comparable value.
  final T value,
) with _$RecursiveBox<T> {
  /// Creates a recursively bounded box.
  this;
}

/// Compound actual arguments survive a nested helper call.
@Derive([CopyWith()])
class const GenericListEnvelope<T>(
  /// A child containing a non-nullable list of nullable elements.
  final GenericBox<List<T?>> child,
) with _$GenericListEnvelope<T> {
  /// Creates a list envelope.
  this;
}
