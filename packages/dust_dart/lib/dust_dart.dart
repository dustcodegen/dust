/// Unified Dart runtime and annotations for Dust code generation.
///
/// The shared `Query` name refers to the Database annotation. Import
/// `package:dust_dart/http.dart` when using the HTTP `Query` annotation.
library;

export 'core.dart';
export 'db.dart';
export 'http.dart' hide Query;
export 'serde.dart';
