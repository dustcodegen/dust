import 'package:dust_dart/derive.dart';

part 'callback_model.g.dart';

/// A non-generic model containing explicitly generic callbacks.
@Derive([CopyWith()])
class CallbackModel with _$CallbackModel {
  /// Creates a model with a required callback and an optional callback.
  const CallbackModel({required this.transform, this.optionalTransform});

  /// Preserves the caller's chosen input and result type.
  final T Function<T>(T) transform;

  /// An optional callback that can be cleared by a copy.
  final T Function<T>(T)? optionalTransform;
}
