import 'package:dust_server/server.dart';
import 'package:test/test.dart';

import '../support.dart';

String _apiKey(Request request) => request.headers['x-api-key'] ?? 'anonymous';

void main() {
  group('RateLimit', () {
    Handler build({
      int maxRequests = 2,
      Duration window = const Duration(minutes: 1),
      int maxKeys = 10,
    }) {
      return (Router()
            ..layer(RateLimit(
              maxRequests: maxRequests,
              window: window,
              keyBy: _apiKey,
              maxKeys: maxKeys,
            ))
            ..route('/work', get((request) async => noContent())))
          .handler;
    }

    test('limits one identity without blocking another', () async {
      final app = build();

      expect((await app(request('GET', '/work'))).statusCode, 204);
      expect((await app(request('GET', '/work'))).statusCode, 204);
      final limited = await app(request('GET', '/work'));
      final other = await app(
        request('GET', '/work', headers: {'x-api-key': 'other'}),
      );

      expect(limited.statusCode, 429);
      expect(limited.headers['retry-after'], '30');
      expect(await limited.readAsString(), '{"error":"too many requests"}');
      expect(other.statusCode, 204);
    });

    test('restores capacity over time', () async {
      final app = build(
        maxRequests: 1,
        window: const Duration(milliseconds: 10),
      );

      expect((await app(request('GET', '/work'))).statusCode, 204);
      expect((await app(request('GET', '/work'))).statusCode, 429);
      await Future<void>.delayed(const Duration(milliseconds: 20));

      expect((await app(request('GET', '/work'))).statusCode, 204);
    });

    test('bounds retained identities and fails closed', () async {
      final app = build(maxRequests: 1, maxKeys: 1);

      expect((await app(request('GET', '/work'))).statusCode, 204);
      final response = await app(
        request('GET', '/work', headers: {'x-api-key': 'other'}),
      );

      expect(response.statusCode, 429);
    });

    test('drops a fully restored identity before admitting a new one',
        () async {
      final app = build(
        maxRequests: 1,
        maxKeys: 1,
        window: const Duration(milliseconds: 10),
      );
      expect((await app(request('GET', '/work'))).statusCode, 204);
      await Future<void>.delayed(const Duration(milliseconds: 20));

      final response = await app(
        request('GET', '/work', headers: {'x-api-key': 'other'}),
      );

      expect(response.statusCode, 204);
    });

    test('validates limits before serving', () {
      expect(
        () => const RateLimit(
          maxRequests: 0,
          window: Duration(seconds: 1),
          keyBy: _apiKey,
        ).toMiddleware(),
        throwsArgumentError,
      );
      expect(
        () => const RateLimit(
          maxRequests: 1,
          window: Duration.zero,
          keyBy: _apiKey,
        ).toMiddleware(),
        throwsArgumentError,
      );
      expect(
        () => const RateLimit(
          maxRequests: 1,
          window: Duration(seconds: 1),
          keyBy: _apiKey,
          maxKeys: 0,
        ).toMiddleware(),
        throwsArgumentError,
      );
    });
  });
}
