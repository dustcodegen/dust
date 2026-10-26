import 'package:test/test.dart';
import '../../example/body_limits.dart' as body_limits;
import '../../example/client_ip.dart' as client_ip;
import '../../example/error_handling.dart' as error_handling;
import 'serve.dart';

/// Error handling, limits, and client identity.
///
/// Split out of one suite per example so each file stays readable; they
/// share the harness in `support.dart`.
void main() {
  group('error_handling', () {
    test('a throw is a 500 that says nothing about what broke', () async {
      // An exception message routinely carries a path, a SQL fragment, or a
      // connection string. Returning it is free reconnaissance.
      final app = await example(error_handling.buildApp(onError: (_, __) {}));

      final response = await app.get('/throws');

      expect(response.statusCode, 500);
      expect(app.object(response), {'error': 'Internal server error'});
      expect(response.body, isNot(contains('hunter2')));
    });

    test('onError sees the detail the client did not', () async {
      final faults = <Object>[];
      final app = await example(
        error_handling.buildApp(onError: (error, _) => faults.add(error)),
      );

      await app.get('/throws');

      expect(faults.single.toString(), contains('hunter2'));
    });

    test('a thrown Rejection keeps its own status', () async {
      final app = await example(error_handling.buildApp(onError: (_, __) {}));

      final response = await app.get('/rejects');

      expect(response.statusCode, 409);
      expect(app.object(response)['error'], 'that name is taken');
    });

    test('a returned Err answers the same as a thrown one', () async {
      final app = await example(error_handling.buildApp(onError: (_, __) {}));

      final thrown = await app.get('/rejects');
      final returned = await app.get('/returns');

      expect(returned.statusCode, thrown.statusCode);
      expect(returned.body, thrown.body);
    });

    test('a Rejection is not reported as a fault', () async {
      // It is a decision, not a bug. Reporting it would bury the real faults.
      final faults = <Object>[];
      final app = await example(
        error_handling.buildApp(onError: (error, _) => faults.add(error)),
      );

      await app.get('/rejects');
      await app.get('/returns');

      expect(faults, isEmpty);
    });
  });

  group('body_limits', () {
    test('a small body is accepted', () async {
      final app = await example(body_limits.buildApp());

      expect(
        (await app.post('/notes', const {'title': 'small'})).statusCode,
        201,
      );
    });

    test('a route tightened below the application ceiling refuses', () async {
      // An extractor asking for 16 KB is not widened by the router's 5 MB.
      final app = await example(body_limits.buildApp());

      final response = await app.send(
        'POST',
        '/notes',
        body: '{"title":"${'a' * 40000}"}',
        headers: const {'content-type': 'application/json'},
      );

      expect(response.statusCode, 413);
      expect(app.object(response)['error'], 'body exceeds 16384 bytes');
    });

    test('the route the ceiling was raised for accepts more', () async {
      final app = await example(body_limits.buildApp());

      final response = await app.send(
        'POST',
        '/avatar',
        body: 'a' * 100000,
        headers: const {'content-type': 'application/octet-stream'},
      );

      expect(response.statusCode, 201);
      expect(app.object(response)['bytes'], 100000);
    });
  });

  group('client_ip', () {
    test('with no proxies the socket address wins, header or not', () async {
      // An unproxied server has no reason to believe X-Forwarded-For, and a
      // default that trusts it is a spoofing hole for everyone who never
      // configured it.
      final app = await example(client_ip.buildApp());

      final response = await app.get(
        '/whoami',
        headers: {'x-forwarded-for': '1.2.3.4'},
      );

      expect(app.object(response)['client'], '127.0.0.1');
    });

    test('with one proxy the rightmost entry is the client', () async {
      final app = await example(client_ip.buildApp(trustedProxies: 1));

      final response = await app.get(
        '/whoami',
        headers: {'x-forwarded-for': '9.9.9.9, 203.0.113.7'},
      );

      expect(app.object(response)['client'], '203.0.113.7');
    });

    test('a spoofed leftmost entry is ignored', () async {
      // Anyone may send X-Forwarded-For: 1.2.3.4, and a proxy appends rather
      // than replaces. Counting from the left trusts whatever the client wrote.
      final app = await example(client_ip.buildApp(trustedProxies: 1));

      final response = await app.get(
        '/whoami',
        headers: {'x-forwarded-for': '1.2.3.4, 203.0.113.7'},
      );

      expect(app.object(response)['client'], '203.0.113.7');
    });

    test('more hops trusted than claimed falls back to the socket', () async {
      final app = await example(client_ip.buildApp(trustedProxies: 3));

      final response = await app.get(
        '/whoami',
        headers: {'x-forwarded-for': '1.2.3.4'},
      );

      expect(app.object(response)['client'], '127.0.0.1');
    });

    test('an absent header falls back to the socket', () async {
      final app = await example(client_ip.buildApp(trustedProxies: 1));

      expect(app.object(await app.get('/whoami'))['client'], '127.0.0.1');
    });
  });
}
