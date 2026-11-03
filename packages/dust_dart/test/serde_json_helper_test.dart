import 'package:dust_dart/serde.dart';
import 'package:test/test.dart';

final class _IntCodec implements SerDeCodec<int, String> {
  const _IntCodec();

  @override
  String serialize(int value) => value.toString();

  @override
  int deserialize(String value) => int.parse(value);
}

void main() {
  group('JsonHelper', () {
    test('reads scalar values', () {
      expect(JsonHelper.as<String>('Ada', 'name', 'String'), 'Ada');
      expect(JsonHelper.as<int>(42, 'age', 'int'), 42);
    });

    test('reports type errors with the json key', () {
      expect(
        () => JsonHelper.as<String>(7, 'name', 'String'),
        throwsA(
          isA<ArgumentError>()
              .having((error) => error.name, 'name', 'name')
              .having((error) => error.invalidValue, 'invalidValue', 7)
              .having(
                (error) => '${error.message}',
                'message',
                'expected String at name',
              ),
        ),
      );
    });

    test('builds nested json paths for collections', () {
      expect(JsonHelper.indexPath('metrics.daily', 2), 'metrics.daily[2]');
      expect(JsonHelper.keyPath('metrics', 'daily'), 'metrics.daily');
    });

    test('reads list and map values', () {
      expect(JsonHelper.asList(<Object?>['a'], 'tags'), <Object?>['a']);
      expect(
        JsonHelper.asMap(<String, Object?>{'id': 1}, 'payload'),
        <String, Object?>{'id': 1},
      );
      expect(
        JsonHelper.asMap(<Object?, Object?>{'id': 1}, 'payload'),
        <String, Object?>{'id': 1},
      );
    });

    test('decodes list set and map values with nested keys', () {
      expect(
        JsonHelper.decodeList<int>(
          <Object?>[1, 2],
          'items',
          (item, itemKey) => JsonHelper.as<int>(item, itemKey, 'int'),
        ),
        <int>[1, 2],
      );
      expect(
        JsonHelper.decodeSet<int>(
          <Object?>[1, 1, 2],
          'items',
          (item, itemKey) => JsonHelper.as<int>(item, itemKey, 'int'),
        ),
        <int>{1, 2},
      );
      expect(
        JsonHelper.decodeMap<int>(
          <String, Object?>{'first': 1, 'second': 2},
          'items',
          (item, itemKey) => JsonHelper.as<int>(item, itemKey, 'int'),
        ),
        <String, int>{'first': 1, 'second': 2},
      );
    });

    test('rejects maps with non-string keys', () {
      final invalid = <Object?, Object?>{1: 'bad'};
      expect(
        () => JsonHelper.asMap(invalid, 'payload'),
        throwsA(
          isA<ArgumentError>()
              .having((error) => error.name, 'name', 'payload')
              .having(
                (error) => error.invalidValue,
                'invalidValue',
                same(invalid),
              ),
        ),
      );
    });

    test('parses string-backed builtins', () {
      expect(
        JsonHelper.asDateTime('2026-06-04T00:00:00Z', 'createdAt'),
        DateTime.parse('2026-06-04T00:00:00Z'),
      );
      expect(
        JsonHelper.asUri('https://example.com', 'url').host,
        'example.com',
      );
      expect(
        JsonHelper.asBigInt('9007199254740993', 'id'),
        BigInt.parse('9007199254740993'),
      );
      expect(
        () => JsonHelper.asDateTime('not-a-date', 'createdAt'),
        throwsA(isA<ArgumentError>()),
      );
      expect(
        () => JsonHelper.asUri('http://[', 'url'),
        throwsA(isA<ArgumentError>()),
      );
      expect(
        () => JsonHelper.asBigInt('nan', 'id'),
        throwsA(isA<ArgumentError>()),
      );
    });

    test('decodes with serde codec', () {
      const codec = _IntCodec();

      expect(JsonHelper.encodeWithCodec<int, Object?>(codec, 42), '42');
      expect(JsonHelper.decodeWithCodec<int, Object?>(codec, '42', 'id'), 42);

      expect(
        JsonHelper.decodeWithCodecResult<int, String>(codec, '42', 'id'),
        const Ok<int, JsonCodecDecodeError>(42),
      );
    });

    test('returns typed codec input failures', () {
      final nullResult = JsonHelper.decodeWithCodecResult<int, String>(
        const _IntCodec(),
        null,
        'id',
      );
      final typeResult = JsonHelper.decodeWithCodecResult<int, String>(
        const _IntCodec(),
        42,
        'id',
      );

      switch (nullResult) {
        case Err(:final JsonDecodeTypeError error):
          expect(error.value, isNull);
          expect(error.path, 'id');
          expect(error.expected, 'non-null value');
        case _:
          fail('expected a typed null failure');
      }
      switch (typeResult) {
        case Err(:final JsonDecodeTypeError error):
          expect(error.value, 42);
          expect(error.path, 'id');
          expect(error.expected, 'String');
        case _:
          fail('expected a typed input failure');
      }
    });

    test('returns codec failures with their path and cause', () {
      final result = JsonHelper.decodeWithCodecResult<int, String>(
        const _IntCodec(),
        'nan',
        'profile.id',
      );

      switch (result) {
        case Err(:final JsonCodecDecodeFailure error):
          expect(error.value, 'nan');
          expect(error.path, 'profile.id');
          expect(error.cause, isA<FormatException>());
          expect(error.message, contains('Custom codec failed at profile.id'));
          expect(error.toString(), error.message);
        case _:
          fail('expected a codec failure');
      }
    });

    test('rejects null codec values', () {
      expect(
        () => JsonHelper.decodeWithCodec<int, Object?>(
          const _IntCodec(),
          null,
          'id',
        ),
        throwsA(isA<ArgumentError>()),
      );
    });

    test('wraps serde codec failures', () {
      expect(
        () => JsonHelper.decodeWithCodec<int, Object?>(
          const _IntCodec(),
          'nan',
          'id',
        ),
        throwsA(isA<ArgumentError>()),
      );
      expect(
        () => JsonHelper.decodeWithCodec<int, String>(
          const _IntCodec(),
          42,
          'id',
        ),
        throwsA(isA<ArgumentError>()),
      );
    });
  });
}
