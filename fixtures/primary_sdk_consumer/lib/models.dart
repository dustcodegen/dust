import 'package:dust_dart/serde.dart';

part 'models.g.dart';

/// Account exercising existing generated APIs with a primary constructor.
@Derive([ToString(), Eq(), CopyWith(), Serialize(), Deserialize(), Validate()])
class const PrimaryAccount({
  /// Database identifier.
  @SerDe(rename: 'account_id') required final int id,

  /// Display name.
  @Validate(length: Length(min: 1)) required final String name,

  /// Optional note.
  final String? note,

  /// Number of visits.
  @SerDe(defaultValue: 0) final int visits = 0,
}) with _$PrimaryAccount {
  /// Creates an account with the declared defaults.
  this;

  /// Reads the generated JSON representation.
  factory PrimaryAccount.fromJson(Map<String, Object?> json) =>
      _$PrimaryAccountFromJson(json);
}

/// A named primary constructor with a mutable declaring parameter.
@Derive([ToString(), CopyWith()])
class PrimaryCounter.named(
  /// Current counter value.
  var int value, [

  /// Label retained by copy operations.
  final String label = 'counter',
]) with _$PrimaryCounter {
  /// Validates construction before the handwritten body runs.
  this : assert(value >= 0, 'value must not be negative');
}
