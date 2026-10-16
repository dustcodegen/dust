import 'package:dust_dart/derive.dart';

import 'generic_models.dart' as models;

part 'generic_imported.g.dart';

/// Imported generic values can be replaced without private helper references.
@Derive([CopyWith()])
class ImportedEnvelope<T> with _$ImportedEnvelope<T> {
  /// Creates an envelope with an imported value.
  const ImportedEnvelope(this.child);

  /// The imported value; chained private helpers remain in its own library.
  final models.GenericBox<T> child;
}
