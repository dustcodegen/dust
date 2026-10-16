import 'package:dust_dart/http.dart';

import 'models.dart';

part 'api.g.dart';

/// The existing HTTP API consumes primary request and response models.
@HttpClient(baseUrl: 'https://example.invalid', generateTest: false)
abstract interface class PrimaryApi {
  /// Creates the generated transport adapter.
  factory PrimaryApi(Dio dio, {String? baseUrl}) = _$PrimaryApi;

  /// Reads an account through its generated primary-model deserializer.
  @GET('/accounts/{id}')
  Future<PrimaryAccount> account(@Path() int id);

  /// Sends the generated primary-model JSON representation.
  @POST('/accounts')
  Future<PrimaryAccount> save(@Body() PrimaryAccount account);
}
