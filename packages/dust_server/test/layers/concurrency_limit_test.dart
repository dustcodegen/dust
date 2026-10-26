import 'dart:async';

import 'package:dust_server/server.dart';
import 'package:test/test.dart';

import '../support.dart';

void main() {
  group('ConcurrencyLimit', () {
    test('never executes more than the configured maximum', () async {
      var active = 0;
      var highest = 0;
      final app = Router()
        ..layer(
          const ConcurrencyLimit(
            maxConcurrentRequests: 2,
            maxQueuedRequests: 6,
          ),
        )
        ..route('/work', get((request) async {
          active++;
          if (active > highest) highest = active;
          await Future<void>.delayed(const Duration(milliseconds: 10));
          active--;
          return noContent();
        }));

      final responses = await Future.wait(
        List.generate(
          8,
          (_) => Future<Response>.value(
            app.handler(request('GET', '/work')),
          ),
        ),
      );

      expect(
          responses.map((response) => response.statusCode), everyElement(204));
      expect(highest, 2);
    });

    test('rejects above a full queue before the handler runs', () async {
      final started = Completer<void>();
      final release = Completer<void>();
      var calls = 0;
      final app = Router()
        ..layer(
          const ConcurrencyLimit(
            maxConcurrentRequests: 1,
            retryAfter: Duration(seconds: 3),
          ),
        )
        ..route('/work', get((request) async {
          calls++;
          started.complete();
          await release.future;
          return noContent();
        }));

      final first = Future<Response>.value(
        app.handler(request('GET', '/work')),
      );
      await started.future;
      final rejected = await app.handler(request('GET', '/work'));
      release.complete();
      await first;

      expect(rejected.statusCode, 503);
      expect(rejected.headers['retry-after'], '3');
      expect(
          await rejected.readAsString(), '{"error":"server is at capacity"}');
      expect(calls, 1);
    });

    test('admits queued requests in arrival order', () async {
      final firstStarted = Completer<void>();
      final releaseFirst = Completer<void>();
      final order = <String>[];
      final app = Router()
        ..layer(
          const ConcurrencyLimit(
            maxConcurrentRequests: 1,
            maxQueuedRequests: 2,
          ),
        )
        ..route('/{id}', get((request) async {
          final id = RequestParts.of(request).pathParameters['id']!;
          order.add(id);
          if (id == '1') {
            firstStarted.complete();
            await releaseFirst.future;
          }
          return noContent();
        }));

      final first = Future<Response>.value(app.handler(request('GET', '/1')));
      await firstStarted.future;
      final second = Future<Response>.value(app.handler(request('GET', '/2')));
      final third = Future<Response>.value(app.handler(request('GET', '/3')));
      releaseFirst.complete();
      await Future.wait([first, second, third]);

      expect(order, ['1', '2', '3']);
    });

    test('a timed-out queued request never enters the handler', () async {
      var calls = 0;
      final app = Router()
        ..layer(const RequestTimeout(Duration(milliseconds: 20)))
        ..layer(
          const ConcurrencyLimit(
            maxConcurrentRequests: 1,
            maxQueuedRequests: 1,
          ),
        )
        ..route('/work', get((request) async {
          calls++;
          await Future<void>.delayed(const Duration(milliseconds: 80));
          return noContent();
        }));

      final responses = await Future.wait([
        Future<Response>.value(app.handler(request('GET', '/work'))),
        Future<Response>.value(app.handler(request('GET', '/work'))),
      ]);
      await Future<void>.delayed(const Duration(milliseconds: 90));

      expect(responses.map((response) => response.statusCode), [503, 503]);
      expect(calls, 1);
    });

    test('validates every limit before serving', () {
      expect(
        () => const ConcurrencyLimit(maxConcurrentRequests: 0).toMiddleware(),
        throwsArgumentError,
      );
      expect(
        () => const ConcurrencyLimit(
          maxConcurrentRequests: 1,
          maxQueuedRequests: -1,
        ).toMiddleware(),
        throwsArgumentError,
      );
      expect(
        () => const ConcurrencyLimit(
          maxConcurrentRequests: 1,
          retryAfter: Duration.zero,
        ).toMiddleware(),
        throwsArgumentError,
      );
    });

    test('holds a slot until a streamed body is consumed', () async {
      final app = Router()
        ..layer(const ConcurrencyLimit(maxConcurrentRequests: 1))
        ..route(
          '/stream',
          get((request) async => Response.ok(Stream.value([1]))),
        );

      final first = await app.handler(request('GET', '/stream'));
      final overloaded = await app.handler(request('GET', '/stream'));
      await first.read().drain<void>();
      final after = await app.handler(request('GET', '/stream'));

      expect(overloaded.statusCode, 503);
      expect(after.statusCode, 200);
      await after.read().drain<void>();
    });
  });
}
