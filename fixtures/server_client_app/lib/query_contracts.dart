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
