import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:http/http.dart' as http;
import 'package:test/test.dart';

import '../../example/webhook_signatures.dart' as webhooks;
import 'serve.dart';

/// Signature, replay, and disclosure behavior for the webhook example.
void main() {
  group('webhook_signatures', () {
    const secret = 'shh';

    String stamp([int offsetSeconds = 0]) =>
        '${DateTime.now().millisecondsSinceEpoch ~/ 1000 + offsetSeconds}';

    String sign(String body, String timestamp) => base64.encode(
          Hmac(
            sha256,
            utf8.encode(secret),
          ).convert(utf8.encode('$timestamp.$body')).bytes,
        );

    Future<http.Response> send(
      ExampleApp app,
      String body, {
      String? signature,
      String? timestamp,
    }) {
      final at = timestamp ?? stamp();
      return app.send(
        'POST',
        '/hooks',
        body: body,
        headers: {
          'content-type': 'application/json',
          'x-timestamp': at,
          if (signature != null) 'x-signature': signature,
        },
      );
    }

    test('a correctly signed request is accepted', () async {
      final app = await example(webhooks.buildApp(secret: secret));
      const body = '{"event":"invoice.paid"}';
      final at = stamp();

      final response = await send(
        app,
        body,
        signature: sign(body, at),
        timestamp: at,
      );

      expect(response.statusCode, 200);
      expect(app.object(response), {'received': 'invoice.paid'});
    });

    test('the same signed request is accepted only once', () async {
      final app = await example(webhooks.buildApp(secret: secret));
      const body = '{"event":"invoice.paid"}';
      final at = stamp();
      final signature = sign(body, at);

      expect(
        (await send(app, body, signature: signature, timestamp: at)).statusCode,
        200,
      );
      expect(
        (await send(app, body, signature: signature, timestamp: at)).statusCode,
        401,
      );
    });

    test('a wrong signature is refused', () async {
      final app = await example(webhooks.buildApp(secret: secret));

      expect(
        (await send(app, '{"event":"x"}', signature: 'wrong')).statusCode,
        401,
      );
    });

    test('a body altered after signing is refused', () async {
      final app = await example(webhooks.buildApp(secret: secret));
      final at = stamp();

      final response = await send(
        app,
        '{"amount":1000000}',
        signature: sign('{"amount":1}', at),
        timestamp: at,
      );

      expect(response.statusCode, 401);
    });

    test('re-serializing the body would break the signature', () async {
      // Same object, different bytes: key order differs. Verifying a re-encoded
      // body compares against something nobody signed.
      final app = await example(webhooks.buildApp(secret: secret));
      const sent = '{"a":1,"b":2}';
      final at = stamp();
      final reordered = jsonEncode(jsonDecode('{"b":2,"a":1}'));

      expect(reordered, isNot(sent));
      expect(
        (await send(
          app,
          reordered,
          signature: sign(sent, at),
          timestamp: at,
        ))
            .statusCode,
        401,
      );
    });

    test(
      'an old timestamp is refused, so a capture cannot be replayed',
      () async {
        final app = await example(webhooks.buildApp(secret: secret));
        const body = '{"event":"x"}';
        final old = stamp(-3600);

        expect(
          (await send(
            app,
            body,
            signature: sign(body, old),
            timestamp: old,
          ))
              .statusCode,
          401,
        );
      },
    );

    test('a timestamp far in the future is refused too', () async {
      final app = await example(webhooks.buildApp(secret: secret));
      const body = '{"event":"x"}';
      final ahead = stamp(3600);

      expect(
        (await send(
          app,
          body,
          signature: sign(body, ahead),
          timestamp: ahead,
        ))
            .statusCode,
        401,
      );
    });

    test('a missing signature is refused', () async {
      final app = await example(webhooks.buildApp(secret: secret));

      expect((await send(app, '{}')).statusCode, 401);
    });

    test('the refusal says nothing about which half failed', () async {
      // Telling an attacker whether the signature or the clock was wrong halves
      // their search.
      final app = await example(webhooks.buildApp(secret: secret));
      final old = stamp(-3600);

      final badSignature = await send(app, '{}', signature: 'wrong');
      final badClock = await send(
        app,
        '{}',
        signature: sign('{}', old),
        timestamp: old,
      );

      expect(badSignature.body, badClock.body);
    });
  });
}
