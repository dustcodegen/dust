import 'package:dust_server/server.dart';
import 'package:server_client_app/query_contracts.dart' as contracts;

part 'client_edges.g.dart';

/// Exercises every wire binding and request mode exposed to generated clients.
@Controller('/transport', tags: ['public'])
class Transport with _$Transport {
  /// Searches with one shared Serde query contract.
  @GET('/search')
  Future<String> search(@Query() contracts.TransportFilter filter) async =>
      '${filter.owner}:${filter.completed}:${filter.page}:'
      '${filter.tags.join(',')}:${filter.order.name}';

  /// Replaces a resource with aggregate query and header values.
  @PUT('/{id}')
  Future<String> replace(
    @Path() String id,
    @Queries() Map<String, String> query,
    @Query('tag') List<String> tags,
    @Header('x-token') String token,
    @HeaderMap() Map<String, String> headers,
  ) async =>
      id;

  /// Updates a resource with a URL-encoded form body.
  @FormUrlEncoded()
  @PATCH('/form')
  Future<String> form(
    @Field('title') String title,
    @Field('priority') String priority,
  ) async =>
      '$title:$priority';

  /// Uploads scalar multipart fields.
  @MultiPart()
  @POST('/multipart')
  Future<String> multipart(
    @Part('title') String title,
    @Part('note') String note,
  ) async =>
      '$title:$note';

  /// Sends a body-free resource probe.
  @HEAD('/{id}')
  Future<void> head(@Path() String id) async {}

  /// Sends a body-free capability probe.
  @OPTIONS('/')
  Future<void> options() async {}
}

/// Proves route-level tags can split one controller across client packages.
@Controller('/operations')
class Operations with _$Operations {
  /// Appears only in the public client.
  @GET('/public', tags: ['public'])
  Future<String> publicOnly() async => 'public';

  /// Appears only in the administrative client.
  @GET('/admin', tags: ['admin'])
  Future<String> adminOnly() async => 'admin';

  /// Appears in both tagged clients.
  @GET('/shared', tags: ['public', 'admin'])
  Future<String> shared() async => 'shared';

  /// Has no client tag and remains server-only.
  @GET('/internal')
  Future<String> internal() async => 'internal';
}
