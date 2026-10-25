import 'dart:async';

import 'package:dust_server/server.dart';
import 'package:test/test.dart';

import '../support.dart';

void main() {
  group('RequestTimeout', () {
    test('lets a fast handler through untouched', () async {
      final app = Router()
        ..layer(const RequestTimeout(Duration(seconds: 5)))
        ..route('/a', get((request) async => textResponse('fast')));

      final response = await app.handler(request('GET', '/a'));

      expect(response.statusCode, 200);
      expect(await response.readAsString(), 'fast');
    });

    test('answers 503 when the budget runs out', () async {
      final app = Router()
        ..layer(const RequestTimeout(Duration(milliseconds: 20)))
        ..route('/slow', get((request) async {
          await Future<void>.delayed(const Duration(seconds: 5));
          return textResponse('too late');
        }));

      final response = await app.handler(request('GET', '/slow'));

      expect(response.statusCode, 503);
      expect(await response.readAsString(), contains('20ms'));
    });

    test('reports the timeout to the callback', () async {
      var reported = 0;
      final app = Router()
        ..layer(RequestTimeout(
          const Duration(milliseconds: 20),
          onTimeout: (request) => reported++,
        ))
        ..route('/slow', get((request) async {
          await Future<void>.delayed(const Duration(seconds: 5));
          return noContent();
        }));

      await app.handler(request('GET', '/slow'));

      expect(reported, 1);
    });

    test('reports a callback failure without replacing the 503', () async {
      final errors = <Object>[];
      final app = Router(onError: (error, stack) => errors.add(error))
        ..layer(RequestTimeout(
          const Duration(milliseconds: 20),
          onTimeout: (_) => throw StateError('metrics failed'),
        ))
        ..route('/slow', get((request) async {
          await Future<void>.delayed(const Duration(seconds: 1));
          return noContent();
        }));

      final response = await app.handler(request('GET', '/slow'));

      expect(response.statusCode, 503);
      expect(errors.single, isA<StateError>());
    });

    test('does not fire for a handler that finishes in time', () async {
      var reported = 0;
      final app = Router()
        ..layer(RequestTimeout(
          const Duration(seconds: 5),
          onTimeout: (request) => reported++,
        ))
        ..route('/a', get((request) async => noContent()));

      await app.handler(request('GET', '/a'));

      expect(reported, 0);
    });

    test('covers a route that was never matched', () async {
      final app = Router()
        ..layer(const RequestTimeout(Duration(seconds: 5)))
        ..route('/a', get((request) async => noContent()));

      expect((await app.handler(request('GET', '/nope'))).statusCode, 404);
    });

    test('signals cooperative cancellation when the budget expires', () async {
      RequestCancellation? cancellation;
      final finished = Completer<void>();
      final app = Router()
        ..layer(const RequestTimeout(Duration(milliseconds: 20)))
        ..route('/slow', get((request) async {
          cancellation = requestCancellationOf(request);
          await cancellation!.whenCancelled;
          finished.complete();
          return noContent();
        }));

      final response = await app.handler(request('GET', '/slow'));
      await finished.future;

      expect(response.statusCode, 503);
      expect(cancellation!.isCancelled, isTrue);
    });

    test('the cancellation extractor works with generated handlers', () async {
      Result<RequestCancellation, Rejection>? result;
      final app = Router()
        ..layer(const RequestTimeout(Duration(seconds: 1)))
        ..route('/a', get((request) async {
          result =
              await const RequestCancellationExtractable().extract(request);
          return noContent();
        }));

      await app.handler(request('GET', '/a'));

      expect(result, isA<Ok<RequestCancellation, Rejection>>());
    });

    test('the cancellation extractor fails clearly without the layer',
        () async {
      final result = await const RequestCancellationExtractable().extract(
        request('GET', '/'),
      );

      expect(result, isA<Err<RequestCancellation, Rejection>>());
      expect(
          requestCancellationOf(
            request('GET', '/',
                context: {'dust_server/request_cancellation': 1}),
          ),
          isNull);
    });

    test('does not mistake a handler TimeoutException for its deadline',
        () async {
      final handler = const RequestTimeout(Duration(seconds: 1))
          .toMiddleware()((request) => throw TimeoutException('dependency'));

      await expectLater(
        handler(request('GET', '/')),
        throwsA(isA<TimeoutException>()),
      );
    });

    test('rejects a non-positive budget before serving', () {
      expect(
        () => const RequestTimeout(Duration.zero).toMiddleware(),
        throwsArgumentError,
      );
    });
  });
}
