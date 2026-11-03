import 'dart:convert';

import '../fp/result.dart';

/// A typed failure returned while decoding JSON.
sealed class JsonDecodeError {
  /// Creates a JSON decode error.
  const JsonDecodeError();

  /// Human-readable failure description.
  String get message;

  @override
  String toString() => message;
}

/// Failure returned while decoding JSON through a custom codec.
sealed class JsonCodecDecodeError {
  /// Creates a custom codec decode error.
  const JsonCodecDecodeError();

  /// Human-readable failure description.
  String get message;

  @override
  String toString() => message;
}

/// Invalid JSON syntax reported by `dart:convert`.
final class JsonDecodeSyntaxError extends JsonDecodeError {
  /// Creates a JSON syntax error.
  const JsonDecodeSyntaxError({required this.source, required this.cause});

  /// Source text that failed to decode.
  final String source;

  /// Original decoder failure retained for compatibility adapters.
  final FormatException cause;

  @override
  String get message => cause.message;
}

/// A decoded JSON value that does not have the expected shape.
final class JsonDecodeTypeError extends JsonDecodeError
    implements JsonCodecDecodeError {
  /// Creates a JSON shape error.
  const JsonDecodeTypeError({
    required this.value,
    required this.path,
    required this.expected,
  });

  /// Decoded value with the wrong shape.
  final Object? value;

  /// JSON path where the mismatch occurred.
  final String path;

  /// Expected JSON shape.
  final String expected;

  @override
  String get message => 'Expected $expected at $path.';
}

/// A custom codec that threw while decoding one JSON value.
final class JsonCodecDecodeFailure extends JsonCodecDecodeError {
  /// Creates a custom codec failure.
  const JsonCodecDecodeFailure({
    required this.value,
    required this.path,
    required this.cause,
  });

  /// JSON value passed to the codec.
  final Object? value;

  /// JSON path where the codec failed.
  final String path;

  /// Original codec failure.
  final Object cause;

  @override
  String get message => 'Custom codec failed at $path: $cause';
}

/// Decodes [source] without throwing for invalid JSON syntax.
Result<Object?, JsonDecodeError> decodeJsonResult(String source) {
  try {
    return Ok<Object?, JsonDecodeError>(jsonDecode(source));
  } on FormatException catch (error) {
    return Err<Object?, JsonDecodeError>(
      JsonDecodeSyntaxError(source: source, cause: error),
    );
  }
}

/// Decodes [source] as a JSON object without throwing for expected failures.
Result<Map<String, Object?>, JsonDecodeError> decodeJsonObjectResult(
  String source,
) {
  return decodeJsonResult(source).andThen(
    (value) => switch (value) {
      Map<String, Object?>() => Ok(value),
      _ => Err(
          JsonDecodeTypeError(
            value: value,
            path: r'$',
            expected: 'Map<String, Object?>',
          ),
        ),
    },
  );
}
