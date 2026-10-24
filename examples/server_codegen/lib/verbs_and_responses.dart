@Routes('/resources', name: 'resourceRoutes')
library;

import 'package:dust_server/server.dart';
import 'package:server_codegen/src/run_example.dart';

part 'verbs_and_responses.g.dart';

/// Runs the remaining generated HTTP verbs and response shapes.
///
/// ```bash
/// dart run lib/verbs_and_responses.dart
/// curl -X PUT localhost:8080/resources/7
/// curl -X PATCH localhost:8080/resources/7
/// curl -i -X DELETE localhost:8080/resources/7
/// ```
Future<void> main() => runExample(buildApp()); // coverage:ignore-line

/// Mounts the generated resource routes.
Router buildApp() => resourceRoutes();

/// Replaces one resource.
@PUT('/{id}', summary: 'Replace a resource', operationId: 'replaceResource')
Future<Map<String, Object?>> replace(@Path() int id) async => {
      'method': 'PUT',
      'id': id,
    };

/// Updates part of one resource.
@PATCH('/{id}')
Future<Map<String, Object?>> update(@Path() int id) async => {
      'method': 'PATCH',
      'id': id,
    };

/// Deletes one resource and becomes a 204 response.
@DELETE('/{id}')
Future<void> delete(@Path() int id) async {}

/// Answers a body-free health probe with an explicit success status.
@HEAD('/{id}', status: 200)
Future<void> head(@Path() int id) async {}

/// Advertises the route without a response body.
@OPTIONS('/', status: 204)
Future<void> options() async {}

/// Lets a successful [Result] become the response.
@GET('/result')
Future<Result<String, Rejection>> result() async => const Ok('ok');

/// Lets an absent [Option] become a 404 response.
@GET('/missing')
Future<Option<String>> missing() async => const None();

/// Preserves a response the handler built itself.
@GET('/custom')
Future<Response> custom() async => Response(202, body: 'custom');
