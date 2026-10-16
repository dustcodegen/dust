import 'package:dust_dart/derive.dart';

part 'generic_value.g.dart';

/// A generic primary model compiled and exercised by Flutter.
@Derive([CopyWith()])
class const GenericValue<T>(
  /// The chosen value.
  final T value,
) with _$GenericValue<T> {
  /// Creates a value.
  this;
}
