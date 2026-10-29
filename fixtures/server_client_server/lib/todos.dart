import 'package:dust_server/annotations.dart';

part 'todos.g.dart';

/// Server routes used to prove backend-derived client generation.
@Controller('/todos', tags: ['public'])
class Todos with _$Todos {
  /// Reads one todo by path and optional query value.
  @GET('/{id}')
  Future<String> read(
    @Path() String id, {
    @Query('q') String? query,
  }) async =>
      '$id:${query ?? ''}';

  /// Creates one todo from a JSON body.
  @POST('/')
  Future<String> create(@Body() Map<String, Object?> body) async =>
      body['title']! as String;

  /// Deletes one todo by path.
  @DELETE('/{id}')
  Future<String> delete(@Path() String id) async => id;
}

/// Administrative routes generated into a separate tagged client package.
@Controller('/admin/todos', tags: ['admin'])
class AdminTodos with _$AdminTodos {
  /// Lists the administrative todo view.
  @GET('/')
  Future<String> list() async => 'admin';
}
