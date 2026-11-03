/// Runtime JSON conversion helpers used by Dust-generated serde code.
///
/// Generated `.g.dart` files are `part of` the user library, so they reuse the
/// app author's existing `package:dust_dart/serde.dart` import instead of
/// emitting duplicate private helpers into every generated file.
library;

import '../fp/result.dart';
import 'json_result.dart';
import 'serde.dart';

/// Shared runtime helpers used by Dust-generated serde code.
abstract final class JsonHelper {
  /// Throws a consistent type error for one JSON key.
  static Never typeError(Object? value, String key, String expected) {
    throw ArgumentError.value(value, key, 'expected $expected at $key');
  }

  /// Reads [value] as [T], or throws a typed JSON error for [key].
  static T as<T>(Object? value, String key, String expected) {
    return value is T ? value : typeError(value, key, expected);
  }

  /// Parses a string-backed scalar with [parse].
  static T parseString<T>(
    Object? value,
    String key,
    String expected,
    T? Function(String value) parse,
  ) {
    return parse(as<String>(value, key, 'String')) ??
        typeError(value, key, expected);
  }

  /// Reads [value] as a JSON list.
  static List<Object?> asList(Object? value, String key) {
    return as<List>(value, key, 'List<Object?>').cast<Object?>();
  }

  /// Decodes a JSON list while extending [key] with each item index.
  static List<T> decodeList<T>(
    Object? value,
    String key,
    T Function(Object? item, String itemKey) decode,
  ) {
    final list = asList(value, key);
    return [
      for (var index = 0; index < list.length; index++)
        decode(list[index], indexPath(key, index)),
    ];
  }

  /// Decodes a JSON set through [decodeList].
  static Set<T> decodeSet<T>(
    Object? value,
    String key,
    T Function(Object? item, String itemKey) decode,
  ) {
    return decodeList(value, key, decode).toSet();
  }

  /// Reads [value] as a `Map<String, Object?>`.
  static Map<String, Object?> asMap(Object? value, String key) {
    final map = as<Map>(value, key, 'Map<String, Object?>');
    return Map.fromEntries(
      map.entries.map(
        (entry) => switch (entry.key) {
          final String mapKey => MapEntry(mapKey, entry.value),
          _ => typeError(value, key, 'Map<String, Object?>'),
        },
      ),
    );
  }

  /// Decodes a JSON map while extending [key] with each map key.
  static Map<String, T> decodeMap<T>(
    Object? value,
    String key,
    T Function(Object? item, String itemKey) decode,
  ) {
    return asMap(value, key).map(
      (mapKey, item) => MapEntry(mapKey, decode(item, keyPath(key, mapKey))),
    );
  }

  /// Returns the JSON path for a list item.
  static String indexPath(String key, int index) => '$key[$index]';

  /// Returns the JSON path for a map entry.
  static String keyPath(String key, String childKey) => '$key.$childKey';

  /// Reads an ISO-8601 [DateTime] string.
  static DateTime asDateTime(Object? value, String key) {
    return parseString(
      value,
      key,
      'ISO-8601 DateTime string',
      DateTime.tryParse,
    );
  }

  /// Reads a URI string.
  static Uri asUri(Object? value, String key) {
    return parseString(value, key, 'Uri string', Uri.tryParse);
  }

  /// Reads a [BigInt] string.
  static BigInt asBigInt(Object? value, String key) {
    return parseString(value, key, 'BigInt string', BigInt.tryParse);
  }

  /// Encodes [value] through a user-provided typed codec.
  static JsonT encodeWithCodec<DartT, JsonT>(
    Serializer<DartT, JsonT> codec,
    DartT value,
  ) {
    return codec.serialize(value);
  }

  /// Decodes [value] through a user-provided typed codec.
  static DartT decodeWithCodec<DartT, JsonT>(
    Deserializer<DartT, JsonT> codec,
    Object? value,
    String key,
  ) {
    return switch (decodeWithCodecResult(codec, value, key)) {
      Ok(:final value) => value,
      Err(:final error) => throw ArgumentError.value(
          value,
          key,
          switch (error) {
            JsonDecodeTypeError(value: null) =>
              'expected value for SerDeCodec at $key',
            JsonCodecDecodeFailure(:final cause) =>
              'failed SerDeCodec decode at $key: $cause',
            JsonDecodeTypeError() =>
              'failed SerDeCodec decode at $key: ${error.message}',
          },
        ),
    };
  }

  /// Decodes [value] through a user-provided codec without throwing for
  /// expected type or codec failures.
  static Result<DartT, JsonCodecDecodeError>
      decodeWithCodecResult<DartT, JsonT>(
    Deserializer<DartT, JsonT> codec,
    Object? value,
    String key,
  ) {
    if (value == null || value is! JsonT) {
      return Err<DartT, JsonCodecDecodeError>(
        JsonDecodeTypeError(
          value: value,
          path: key,
          expected: value == null ? 'non-null value' : '$JsonT',
        ),
      );
    }
    // The guard establishes this; Dart does not promote to a type variable.
    final typedValue = value as JsonT;
    try {
      return Ok<DartT, JsonCodecDecodeError>(codec.deserialize(typedValue));
    } catch (error) {
      return Err<DartT, JsonCodecDecodeError>(
        JsonCodecDecodeFailure(value: value, path: key, cause: error),
      );
    }
  }
}
