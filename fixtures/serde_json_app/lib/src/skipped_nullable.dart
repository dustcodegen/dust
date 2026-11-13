import 'package:dust_dart/serde.dart';

part 'skipped_nullable.g.dart';

/// Nullable local state is initialized without reading excluded JSON keys.
@Derive([Serialize(), Deserialize()])
final class NullableSkipped with _$NullableSkipped {
  /// Creates a model with both explicit and implicit Dart defaults.
  NullableSkipped({
    required this.requiredLocal,
    this.both,
    this.decodeOnly,
    this.optionalDefault = 'constructor-default',
    this.explicitValue = 'constructor-explicit',
    this.ordinary = 'ordinary-default',
  });

  /// Decodes the public wire fields.
  factory NullableSkipped.fromJson(Map<String, Object?> json) =>
      _$NullableSkippedFromJson(json);

  /// A required nullable argument still receives null during decoding.
  @SerDe(skip: true)
  final String? requiredLocal;

  /// Local state omitted in both directions.
  @SerDe(skip: true)
  final String? both;

  /// Local state that can be serialized but never populated from JSON.
  @SerDe(skipDeserializing: true)
  final String? decodeOnly;

  /// The optional constructor's non-null default is retained.
  @SerDe(skip: true)
  final String? optionalDefault;

  /// An explicit SerDe default takes precedence over the constructor default.
  @SerDe(skip: true, defaultValue: 'serde-default')
  final String? explicitValue;

  /// A field initializer remains owned by the handwritten class.
  @SerDe(skip: true)
  final String? initialized = 'field-default';

  /// Ordinary missing-key behavior is independent of skipped-field defaults.
  final String? ordinary;
}

/// Positional defaults survive when later decoded arguments are present.
@Derive([Serialize(), Deserialize()])
final class PositionalSkipped with _$PositionalSkipped {
  /// Creates a positional model.
  const PositionalSkipped([this.local = 'position-default', this.count = 7]);

  /// Decodes the count while leaving local state at its Dart default.
  factory PositionalSkipped.fromJson(Map<String, Object?> json) =>
      _$PositionalSkippedFromJson(json);

  /// Local state ignored during decoding.
  @SerDe(skipDeserializing: true)
  final String? local;

  /// The later argument forces the decoder to fill the preceding position.
  final int count;
}

/// Strict-key mode keeps rejecting excluded input keys.
@Derive([Serialize(), Deserialize()])
@SerDe(disallowUnrecognizedKeys: true)
final class StrictSkipped with _$StrictSkipped {
  /// Creates an empty wire model.
  const StrictSkipped({this.local});

  /// Decodes only an empty object.
  factory StrictSkipped.fromJson(Map<String, Object?> json) =>
      _$StrictSkippedFromJson(json);

  /// Local state is not a recognized wire key.
  @SerDe(skip: true)
  final String? local;
}
