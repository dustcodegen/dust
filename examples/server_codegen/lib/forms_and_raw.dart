@Routes('/bodies', name: 'bodyRoutes')
library;

import 'dart:typed_data';

import 'package:dust_server/server.dart';
import 'package:server_codegen/src/run_example.dart';

part 'forms_and_raw.g.dart';

/// Runs generated form and raw-body extraction.
///
/// ```bash
/// dart run lib/forms_and_raw.dart
/// curl -X POST localhost:8080/bodies/form \
///   -H 'content-type: application/x-www-form-urlencoded' \
///   -d 'title=Ship+Dust&priority=2'
/// curl -X POST localhost:8080/bodies/text -d 'plain text'
/// ```
Future<void> main() => runExample(buildApp()); // coverage:ignore-line

/// Mounts the generated body routes.
Router buildApp() => bodyRoutes();

/// Reads several URL-encoded fields from one body pass.
@FormUrlEncoded()
@POST('/form')
Future<Map<String, Object?>> form(
  @Field('title') String title,
  @Field('priority') Option<int> priority,
) async =>
    {
      'title': title,
      'priority': priority.match(some: (value) => value, none: () => null),
    };

/// Reads raw UTF-8 text.
@POST('/text')
Future<String> text(@RawBody() String body) async => body;

/// Reads raw bytes without decoding them.
@POST('/bytes', status: 202)
Future<Uint8List> bytes(@RawBody() Uint8List body) async => body;

/// Passes an unread request stream through to the response.
@POST('/stream')
Future<Stream<List<int>>> stream(
  @RawBody() Stream<List<int>> body,
) async =>
    body;
