/// One element or action in a parsed Dust UI document.
final class UiNode {
  /// Creates a node with one-based source coordinates.
  UiNode({
    required this.name,
    required this.line,
    required this.column,
    Map<String, Object?> props = const {},
    List<UiNode> children = const [],
    Map<String, ({int line, int column})> propertyLocations = const {},
  }) : props = Map.unmodifiable(props),
       children = List.unmodifiable(children),
       propertyLocations = Map.unmodifiable(propertyLocations);

  /// Element or action name.
  final String name;

  /// Named properties.
  final Map<String, Object?> props;

  /// One-based source coordinates for explicitly supplied properties.
  final Map<String, ({int line, int column})> propertyLocations;

  /// Nested visual elements, or content of an action such as popup.
  final List<UiNode> children;

  /// One-based source line.
  final int line;

  /// One-based source column in UTF-16 code units.
  final int column;

  /// Copies the node with a new child list.
  UiNode withChildren(List<UiNode> children) => UiNode(
    name: name,
    props: props,
    children: children,
    propertyLocations: propertyLocations,
    line: line,
    column: column,
  );
}

/// A typed event property containing supported actions.
final class UiActionBlock {
  /// Creates an immutable action list.
  UiActionBlock(List<UiNode> actions) : actions = List.unmodifiable(actions);

  /// Actions in source order.
  final List<UiNode> actions;
}

/// An unquoted identifier, such as the enum value `popup`.
final class UiSymbol {
  /// Creates an identifier value.
  const UiSymbol(this.name);

  /// Identifier spelling.
  final String name;
}

/// A read from the nearest local state scope.
final class UiStateRef {
  /// Creates a state reference, optionally with Boolean negation.
  const UiStateRef(this.name, {this.negated = false});

  /// State key.
  final String name;

  /// Whether to negate the referenced Boolean value.
  final bool negated;
}

/// A source-positioned syntax or validation error.
final class UiSyntaxException implements Exception {
  /// Creates a diagnostic with one-based coordinates.
  const UiSyntaxException(this.message, this.line, this.column);

  /// Human-readable explanation.
  final String message;

  /// One-based source line.
  final int line;

  /// One-based source column in UTF-16 code units.
  final int column;

  @override
  String toString() => '$line:$column: $message';
}

/// Maximum nesting of elements or literal values in a document.
const maxUiDepth = 32;

/// Maximum number of elements and actions in a document.
const maxUiNodes = 512;
