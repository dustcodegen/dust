import 'package:dust_dart/serde.dart';

part 'language_models.g.dart';

/// An enum whose primary field does not change its serialized variant names.
@Derive([Serialize(), Deserialize()])
enum PrimaryStatus(final String label) {
  /// An active record.
  @SerDe(rename: 'enabled')
  active('Active'),

  /// An inactive record.
  inactive('Inactive'),
}

/// A primary model keeps enum defaults and generated variant serialization.
@Derive([Eq(), CopyWith(), Serialize(), Deserialize()])
class const StatusRecord({
  /// Record identifier.
  required final int id,

  /// Current state.
  final PrimaryStatus status = PrimaryStatus.active,
}) with _$StatusRecord {
  /// Creates a record through the unnamed constructor.
  this;

  /// Reads the generated representation.
  factory StatusRecord.fromJson(Map<String, Object?> json) =>
      _$StatusRecordFromJson(json);
}

/// An extension-type representation retains ordinary Dart semantics.
extension type const RecordId(int value) {
  /// Creates an identifier without an additional runtime wrapper.
  this;
}

mixin class _PrimaryMarker();

/// A primary model can apply a mixin class with a trivial primary constructor.
@Derive([ToString()])
class const TaggedRecord(final String tag) with _PrimaryMarker, _$TaggedRecord {
  /// Creates a tagged record.
  this;
}
