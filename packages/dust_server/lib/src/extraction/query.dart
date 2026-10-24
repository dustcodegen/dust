import 'package:dust_dart/fp.dart';
import 'package:shelf/shelf.dart';

import '../request/coercion.dart';
import '../response/rejection.dart';
import '../response/validation.dart';
import 'extractable.dart';

/// Extracts one query value, coerced to [T].
///
/// A nullable [T] makes the value optional; a non-nullable [T] rejects with 400
/// when the key is absent.
final class QueryExtractable<T> implements FromRequestParts<T> {
  /// Reads the query value named [key].
  const QueryExtractable(this.key);

  /// The query key.
  final String key;

  @override
  Future<Result<T, Rejection>> extract(Request request) async {
    final raw = request.url.queryParameters[key];
    if (raw == null) {
      if (null is T) return Ok(null as T);
      return Err(Rejection.badRequest('query "$key" is required'));
    }
    return coerce<T>(raw, source: 'query "$key"');
  }
}

/// Extracts every value for one repeated query key, coerced to [T].
///
/// `Uri.queryParameters` keeps only the last value for a repeated key, so
/// reading `?tag=a&tag=b` needs this instead of [QueryExtractable]. An absent
/// key gives an empty list rather than a rejection.
final class QueryListExtractable<T> implements FromRequestParts<List<T>> {
  /// Reads every value for [key].
  const QueryListExtractable(this.key);

  /// The query key.
  final String key;

  @override
  Future<Result<List<T>, Rejection>> extract(Request request) async {
    final raw = request.url.queryParametersAll[key] ?? const <String>[];
    final values = <T>[];
    for (final value in raw) {
      switch (coerce<T>(value, source: 'query "$key"')) {
        case Err(:final error):
          return Err(error);
        case Ok(value: final coerced):
          values.add(coerced);
      }
    }
    return Ok(values);
  }
}

/// Query pairs prepared for a generated `Deserialize()` model.
///
/// Generated code starts with [toObjectMap], then promotes fields whose JSON
/// representation is not a string or collects repeated values into lists.
final class QueryParameters {
  /// Wraps every decoded query value.
  const QueryParameters(this.values);

  /// Query values in URI order, including repeated keys.
  final Map<String, List<String>> values;

  /// Builds the JSON-shaped map consumed by a generated model decoder.
  Map<String, Object?> toObjectMap() => {
        for (final MapEntry(:key, :value) in values.entries)
          if (value.isNotEmpty) key: value.last,
      };

  /// Coerces one present query value to [T].
  void coerceOne<T>(
    Map<String, Object?> target,
    String key, {
    List<String> aliases = const [],
  }) {
    final selected = _selectedKey(key, aliases);
    if (selected == null) return;
    final raw = values[selected];
    if (raw == null || raw.isEmpty) return;
    switch (coerce<T>(raw.last, source: 'query "$selected"')) {
      case Err(:final error):
        throw error;
      case Ok(:final value):
        target[selected] = value;
    }
  }

  /// Coerces every present value for [key] to [T].
  void coerceAll<T>(
    Map<String, Object?> target,
    String key, {
    List<String> aliases = const [],
  }) {
    final selected = _selectedKey(key, aliases);
    if (selected == null) return;
    final raw = values[selected];
    if (raw == null) return;
    final decoded = <T>[];
    for (final item in raw) {
      switch (coerce<T>(item, source: 'query "$selected"')) {
        case Err(:final error):
          throw error;
        case Ok(:final value):
          decoded.add(value);
      }
    }
    target[selected] = decoded;
  }

  /// Copies every present value for [key] into a JSON list.
  void copyAll(
    Map<String, Object?> target,
    String key, {
    List<String> aliases = const [],
  }) {
    final selected = _selectedKey(key, aliases);
    if (selected == null) return;
    final raw = values[selected];
    if (raw != null) target[selected] = List<String>.of(raw);
  }

  /// Selects the same primary-or-alias key as generated SerDe.
  String? _selectedKey(String key, List<String> aliases) {
    if (values.containsKey(key)) return key;
    for (final alias in aliases) {
      if (values.containsKey(alias)) return alias;
    }
    return null;
  }
}

/// Extracts a typed model from the complete query string.
final class QueryModelExtractable<T> implements FromRequestParts<T> {
  /// Creates a model with the generated [deserialize] adapter.
  const QueryModelExtractable(this.deserialize);

  /// Builds the model from prepared query parameters.
  final T Function(QueryParameters query) deserialize;

  @override
  Future<Result<T, Rejection>> extract(Request request) async {
    try {
      return Ok(deserialize(QueryParameters(request.url.queryParametersAll)));
    } on Rejection catch (error) {
      return Err(error);
    } on Object catch (error) {
      return Err(
        Rejection.badRequest(
          'query parameters do not match the expected shape: $error',
        ),
      );
    }
  }
}

/// Extracts every query value as a map.
///
/// Generated `@Queries() Map<String, String>` parameters take the values as-is.
/// A custom decoder can adapt the map for compatibility.
final class QueriesExtractable<T> implements FromRequestParts<T> {
  /// Decodes the query map with [deserialize].
  const QueriesExtractable(this.deserialize);

  /// Builds the value from the collected query pairs.
  final T Function(Map<String, String> queries) deserialize;

  @override
  Future<Result<T, Rejection>> extract(Request request) async {
    try {
      return Ok(deserialize(request.url.queryParameters));
    } on Object catch (error) {
      return Err(decodeRejection(error, subject: 'query parameters'));
    }
  }
}

/// Extracts the raw, undecoded query string.
final class RawQueryExtractable implements FromRequestParts<String?> {
  /// Reads the query component of the request URI.
  const RawQueryExtractable();

  @override
  Future<Result<String?, Rejection>> extract(Request request) async {
    final query = request.url.query;
    return Ok(query.isEmpty ? null : query);
  }
}
