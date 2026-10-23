@Routes('/notes', name: 'noteRoutes')
library;

import 'package:dust_dart/serde.dart';
import 'package:dust_server/server.dart';
import 'package:server_codegen/src/run_example.dart';

part 'json_body.g.dart';

/// Runs generated JSON decoding and validation.
///
/// ```bash
/// dart run lib/json_body.dart
/// curl -X POST localhost:8080/notes -H 'content-type: application/json' \
///   -d '{"title":"Ship Dust"}'
/// curl -i -X POST localhost:8080/notes -H 'content-type: application/json' \
///   -d '{"title":""}'
/// ```
Future<void> main() => runExample(buildApp()); // coverage:ignore-line

/// Mounts the generated route group.
Router buildApp() => noteRoutes();

/// Creates one validated note from JSON.
@POST('/', status: 201)
Future<Map<String, Object?>> createNote(@Body() CreateNote input) async => {
      'title': input.title,
    };

/// The JSON body accepted by [createNote].
@Derive([Validate()])
final class CreateNote with _$CreateNote {
  /// Creates a note request.
  const CreateNote({required this.title});

  /// Decodes one JSON object.
  factory CreateNote.deserialize(Map<String, Object?> json) {
    return CreateNote(title: json['title']! as String);
  }

  /// The required, non-empty note title.
  @Validate(length: Length(min: 1), message: 'must not be empty')
  final String title;
}
