import 'package:dust_server/annotations.dart';

part 'todos.g.dart';

@Controller('/todos')

/// Server routes used to prove backend-derived client generation.
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
