/// Format-neutral documentation facts attached to one generated route.
///
/// The router carries this value but never interprets it. Documentation and
/// inspection packages can recover it through `MountedRoute.metadataOf`.
final class RouteMetadata {
  /// Describes one generated route.
  const RouteMetadata({
    required this.operationId,
    this.summary,
    this.description,
    this.tags = const [],
    this.deprecated = false,
    this.hidden = false,
  });

  /// A stable operation identifier.
  final String operationId;

  /// A one-line summary.
  final String? summary;

  /// A longer description.
  final String? description;

  /// Tags inherited from the route group followed by route-local tags.
  final List<String> tags;

  /// Whether the operation is deprecated.
  final bool deprecated;

  /// Whether documentation tools should hide the operation.
  final bool hidden;
}
