import 'package:dust_server/server.dart';
import 'package:test/test.dart';

import '../support.dart';

void main() {
  test('productionRouter applies the shared runtime policy', () async {
    final records = <AccessRecord>[];
    final errors = <Object>[];
    final app = productionRouter(
      bodyLimit: 32,
      maxConcurrentRequests: 2,
      requestTimeout: const Duration(seconds: 1),
      onRecord: records.add,
      onError: (error, stack) => errors.add(error),
    )
      ..route('/ok', get((request) async => textResponse('ok')))
      ..route('/fail', get((request) async => throw StateError('broken')));

    final ok = await app.handler(
      request('GET', '/ok', headers: {'x-request-id': 'policy-test'}),
    );
    final failed = await app.handler(request('GET', '/fail'));

    expect(ok.headers['x-request-id'], 'policy-test');
    expect(ok.headers['x-content-type-options'], 'nosniff');
    expect(failed.statusCode, 500);
    expect(records.map((record) => record.status), [200, 500]);
    expect(errors.single, isA<StateError>());
  });

  test('productionRouter rejects a negative body limit', () {
    expect(
      () => productionRouter(
        bodyLimit: -1,
        maxConcurrentRequests: 1,
        requestTimeout: const Duration(seconds: 1),
        onRecord: (_) {},
        onError: (_, __) {},
      ),
      throwsArgumentError,
    );
  });
}
