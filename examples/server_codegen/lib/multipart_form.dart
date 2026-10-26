@Routes('/upload', name: 'uploadRoutes')
library;

import 'package:dust_server/server.dart';
import 'package:server_codegen/src/run_example.dart';

part 'multipart_form.g.dart';

/// Runs generated multipart form extraction.
///
/// ```bash
/// dart run lib/multipart_form.dart
/// curl -X POST localhost:8080/upload \
///   -F 'caption=my file' -F 'file=@README.md'
/// ```
Future<void> main() => runExample(buildApp()); // coverage:ignore-line

/// Mounts the generated route group.
Router buildApp() => uploadRoutes();

/// Reads one text field and one buffered file.
@MultiPart()
@POST('/')
Future<Map<String, Object?>> upload(
  @Part('caption') String caption,
  @Part('file') UploadedFile file,
) async =>
    {
      'caption': caption,
      'filename': file.filename,
      'bytes': file.bytes.length,
    };
