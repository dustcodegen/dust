import 'package:dust_dart/serde.dart';

part 'query_contracts.g.dart';

/// Sort order shared by the generated server and client.
@Derive([Serialize(), Deserialize()])
enum TransportOrder {
  /// Oldest values first.
  oldest,

  /// Newest values first.
  newest,
}

/// Typed query contract shared by the generated server and client.
@Derive([Serialize(), Deserialize()])
@SerDe(disallowUnrecognizedKeys: true)
final class TransportFilter with _$TransportFilter {
  /// Creates a typed transport filter.
  const TransportFilter({
    required this.owner,
    this.completed,
    this.page = 1,
    this.tags = const [],
    this.order = TransportOrder.newest,
  });

  /// Owner identifier.
  final String owner;

  /// Optional completion state with a decode-only legacy alias.
  @SerDe(rename: 'done', aliases: ['completed'])
  final bool? completed;

  /// One-based page number.
  @SerDe(defaultValue: 1)
  final int page;

  /// Repeated tag values.
  @SerDe(defaultValue: <String>[])
  final List<String> tags;

  /// Selected sort order.
  @SerDe(defaultValue: TransportOrder.newest)
  final TransportOrder order;
}

/// Nested payload data shared by typed requests and responses.
@Derive([Serialize(), Deserialize()])
final class TransportMetadata with _$TransportMetadata {
  /// Creates nested transport metadata.
  const TransportMetadata({required this.source});

  /// Origin of the transport payload.
  final String source;
}

/// Validated request body shared by the server and generated client.
@Derive([Serialize(), Deserialize(), Validate()])
@SerDe(disallowUnrecognizedKeys: true)
final class CreateTransport with _$CreateTransport {
  /// Creates a transport request.
  const CreateTransport({
    required this.title,
    required this.order,
    required this.metadata,
    this.tags = const [],
  });

  /// User-visible title with a decode-only legacy alias.
  @SerDe(rename: 'display_name', aliases: ['title'])
  @Validate(length: Length(min: 1), message: 'must not be empty')
  final String title;

  /// Selected ordering mode.
  final TransportOrder order;

  /// Nested payload value.
  final TransportMetadata metadata;

  /// Repeated payload labels.
  @SerDe(defaultValue: <String>[])
  final List<String> tags;
}

/// Typed response body shared by the server and generated client.
@Derive([Serialize(), Deserialize()])
final class TransportRecord with _$TransportRecord {
  /// Creates a transport response.
  const TransportRecord({
    required this.id,
    required this.title,
    required this.order,
    required this.metadata,
    this.tags = const [],
  });

  /// Stable record identifier.
  final String id;

  /// User-visible title.
  @SerDe(rename: 'display_name', aliases: ['title'])
  final String title;

  /// Selected ordering mode.
  final TransportOrder order;

  /// Nested payload value.
  final TransportMetadata metadata;

  /// Repeated payload labels.
  @SerDe(defaultValue: <String>[])
  final List<String> tags;
}
