@Routes('/items', name: 'basicRoutes')
library;

import 'package:dust_server/server.dart';

part 'basic_routes.g.dart';

/// Reads one item using a path parameter and optional query parameter.
@GET('/{id}')
Future<Map<String, Object?>> readItem(
  @Path() int id, {
  @Query('details') bool? details,
}) async =>
    {'id': id, 'details': details ?? false};

/// Creates one item from a required query parameter.
@POST('/', status: 201)
Future<Map<String, Object?>> createItem(@Query('name') String name) async =>
    {'name': name};

/// Deletes one item and returns no content.
@DELETE('/{id}')
Future<void> deleteItem(@Path() int id) async {}
