import '../fp/result.dart';
import '../serde/json_result.dart';

/// Decodes a JSON object stored in a text column.
Map<String, Object?> decodeJsonObject(String source) {
  return switch (decodeJsonObjectResult(source)) {
    Ok(:final value) => value,
    Err(error: JsonDecodeSyntaxError(:final cause)) => throw cause,
    Err(error: JsonDecodeTypeError()) =>
      throw FormatException('Expected a JSON object column value.', source),
  };
}
