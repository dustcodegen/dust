import 'package:dust_dart/serde.dart';
import 'package:test/test.dart';

void main() {
  group('decodeJsonResult', () {
    test('returns decoded JSON values', () {
      switch (decodeJsonResult('null')) {
        case Ok(:final value):
          expect(value, isNull);
        case Err():
          fail('expected JSON null to return Ok');
      }
      switch (decodeJsonResult('[1, true]')) {
        case Ok(:final value):
          expect(value, <Object?>[1, true]);
        case Err():
          fail('expected valid JSON to return Ok');
      }
    });

    test('returns syntax errors without throwing', () {
      final result = decodeJsonResult('{');

      switch (result) {
        case Ok():
          fail('expected invalid JSON to return Err');
        case Err(:final JsonDecodeSyntaxError error):
          expect(error.source, '{');
          expect(error.cause, isA<FormatException>());
          expect(error.message, isNotEmpty);
          expect(error.toString(), error.message);
        case Err():
          fail('expected a syntax error');
      }
    });
  });

  group('decodeJsonObjectResult', () {
    test('returns decoded JSON objects', () {
      switch (decodeJsonObjectResult('{"id":1}')) {
        case Ok(:final value):
          expect(value, <String, Object?>{'id': 1});
        case Err():
          fail('expected a JSON object to return Ok');
      }
    });

    test('distinguishes a non-object root from invalid syntax', () {
      final result = decodeJsonObjectResult('[1]');

      switch (result) {
        case Ok():
          fail('expected a non-object root to return Err');
        case Err(:final JsonDecodeTypeError error):
          expect(error.value, <Object?>[1]);
          expect(error.path, r'$');
          expect(error.expected, 'Map<String, Object?>');
          expect(error.message, r'Expected Map<String, Object?> at $.');
          expect(error.toString(), error.message);
        case Err():
          fail('expected a type error');
      }
    });
  });
}
