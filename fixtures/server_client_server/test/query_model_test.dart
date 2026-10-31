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

  test(
    'imported typed bodies deserialize validate and serialize responses',
    () async {
      final router = Transport().routes();

      final valid = await router(
        Request(
          'POST',
          Uri.parse('http://localhost/transport/typed'),
          headers: {'content-type': 'application/json'},
          body: '''
{"display_name":"Ship Dust","order":"newest","metadata":{"source":"client"},"tags":["dart","rust"]}
''',
        ),
      );
      final alias = await router(
        Request(
          'POST',
          Uri.parse('http://localhost/transport/typed'),
          headers: {'content-type': 'application/json'},
          body: '{'
              '"title":"Legacy",'
              '"order":"oldest",'
              '"metadata":{"source":"client"}'
              '}',
        ),
      );
      final invalid = await router(
        Request(
          'POST',
          Uri.parse('http://localhost/transport/typed'),
          headers: {'content-type': 'application/json'},
          body: '{'
              '"display_name":"",'
              '"order":"newest",'
              '"metadata":{"source":"client"}'
              '}',
        ),
      );
      final malformed = await router(
        Request(
          'POST',
          Uri.parse('http://localhost/transport/typed'),
          headers: {'content-type': 'application/json'},
          body: '{',
        ),
      );
      final unknown = await router(
        Request(
          'POST',
          Uri.parse('http://localhost/transport/typed'),
          headers: {'content-type': 'application/json'},
          body: '''
{"display_name":"Dust","order":"newest","metadata":{"source":"client"},"extra":true}
''',
        ),
      );

      expect(valid.statusCode, 201);
      expect(
        await valid.readAsString(),
        contains('"display_name":"Ship Dust"'),
      );
      expect(await alias.readAsString(), contains('"display_name":"Legacy"'));
      expect(invalid.statusCode, 422);
      expect(await invalid.readAsString(), contains('must not be empty'));
      expect(malformed.statusCode, 400);
      expect(unknown.statusCode, 422);
    },
  );

  test(
    'typed response lists and nulls serialize through generated routes',
    () async {
      final router = Transport().routes();
      final listed = await router(
        Request('GET', Uri.parse('http://localhost/transport/typed')),
      );
      final missing = await router(
        Request('GET', Uri.parse('http://localhost/transport/typed/missing')),
      );

      expect(await listed.readAsString(), contains('"display_name":"First"'));
      expect(missing.statusCode, 204);
      expect(await missing.readAsString(), isEmpty);
    },
  );
}
