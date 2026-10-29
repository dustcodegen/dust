import 'package:dust_server/server.dart';
import 'package:server_client_server/client_edges.dart';
import 'package:test/test.dart';

void main() {
  test(
    'imported Serde query contracts decode through generated routes',
    () async {
      final router = Transport().routes();

      final valid = await router(
        Request(
          'GET',
          Uri.parse(
            'http://localhost/transport/search?owner=dust&done=true&page=2&tags=rust&tags=dart&order=oldest',
          ),
        ),
      );
      final alias = await router(
        Request(
          'GET',
          Uri.parse(
            'http://localhost/transport/search?owner=dust&completed=false',
          ),
        ),
      );
      final malformed = await router(
        Request(
          'GET',
          Uri.parse('http://localhost/transport/search?owner=dust&page=many'),
        ),
      );
      final unknown = await router(
        Request(
          'GET',
          Uri.parse('http://localhost/transport/search?owner=dust&extra=value'),
        ),
      );

      expect(await valid.readAsString(), 'dust:true:2:rust,dart:oldest');
      expect(await alias.readAsString(), 'dust:false:1::newest');
      expect(malformed.statusCode, 400);
      expect(unknown.statusCode, 400);
    },
  );
}
