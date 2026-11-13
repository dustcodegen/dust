import 'package:dust_dart/serde.dart';

part 'skipped_nullable.g.dart';

/// Primary constructor fields share the ordinary skipped-field contract.
@Derive([Serialize(), Deserialize()])
class const PrimarySkipped({
  /// A required nullable argument receives null during decoding.
  @SerDe(skip: true) required final String? requiredLocal,

  /// Optional local state defaults to null.
  @SerDe(skip: true) final String? local,

  /// The primary constructor's non-null default is retained.
  @SerDe(skipDeserializing: true) final String? cached = 'primary-default',
}) with _$PrimarySkipped {
  /// Creates a primary-constructor model.
  this;

  /// Decodes without accepting local state from JSON.
  factory PrimarySkipped.fromJson(Map<String, Object?> json) =>
      _$PrimarySkippedFromJson(json);
}
