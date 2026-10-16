import 'package:dust_dart/derive.dart';

import 'generic_models.dart' as models;

part 'generic_imported.g.dart';

/// An imported value is replaceable without cross-library private helpers.
@Derive([CopyWith()])
class const ImportedEnvelope<T>(
  /// The imported value.
  final models.GenericBox<T> child,
) with _$ImportedEnvelope<T> {
  /// Creates an envelope.
  this;
}
