@Routes('/request-data', name: 'requestDataRoutes')
library;

import 'package:dust_server/server.dart';
import 'package:server_codegen/src/run_example.dart';

part 'request_data.g.dart';

/// Runs generated query, header, peer, and request-parts extraction.
///
/// ```bash
/// dart run lib/request_data.dart
/// curl 'localhost:8080/request-data/inspect?tag=red&tag=blue&mode=full' \
///   -H 'x-request-id: demo-1'
/// ```
Future<void> main() => runExample(buildApp()); // coverage:ignore-line

/// Mounts the generated routes below the same timeout used in production.
Router buildApp() => Router()
  ..layer(const RequestTimeout(Duration(seconds: 5)))
  ..merge(requestDataRoutes());

/// Returns the request values generated code extracted before this call.
@GET('/inspect', summary: 'Inspect request data', tags: ['diagnostics'])
Future<Map<String, Object?>> inspect(
  @Query('tag') List<String> tags,
  @RawQuery() String? rawQuery,
  @Header('x-request-id') String? requestId,
  @HeaderMap() Map<String, String> headers,
  @Peer() PeerInfo peer,
  @Extract(RequestCancellationExtractable) RequestCancellation cancellation,
  RequestParts parts,
) async =>
    {
      'tags': tags,
      'query': parts.url.queryParameters,
      'rawQuery': rawQuery,
      'requestId': requestId,
      'host': headers['host'],
      'peer': peer.remoteAddress,
      'method': parts.method,
      'cancelled': cancellation.isCancelled,
    };
