import 'catalog.dart';
import 'model.dart';

part 'controls.dart';

/// Validates a parsed or recomposed UI tree against the prototype catalog.
///
/// Fragments defer target existence checks in their outer surface until merged.
void validateUi(UiNode document, {bool fragment = false}) {
  final validator = _Validator();
  final surface = _Surface();
  validator.visual(document, const {}, surface, 0);
  if (!fragment) validator.targets(surface);
}

final class _Validator {
  int _nodes = 0;

  void visual(
    UiNode node,
    Map<String, Object?> parentState,
    _Surface surface,
    int depth, {
    _Form? form,
    String? parent,
    bool buttonContent = false,
  }) {
    _visit(node, depth);
    final spec =
        visualProperties[node.name] ??
        _fail(node, 'Unsupported element "${node.name}".');
    if (buttonContent &&
        (!const {
              'col',
              'row',
              'zstack',
              'text',
              'spacer',
            }.contains(node.name) ||
            node.props.containsKey('state'))) {
      _fail(node, 'Button content must contain only noninteractive visuals.');
    }
    final declared = node.props['state'];
    final state = declared is Map<String, Object?> ? declared : parentState;
    _properties(node, spec, state);
    final id = node.props['id'];
    if (id is String && !surface.ids.add(id)) {
      _fail(node, 'Duplicate ID "$id".', property: 'id');
    }
    final target = node.props['target'];
    if (target is String) surface.targets.add((node: node, id: target));

    switch (node.name) {
      case 'text':
        _require(node, const ['value']);
        _leaf(node);
      case 'spacer':
        _leaf(node);
        if (parent != 'row') _fail(node, 'spacer must be a child of row.');
      case 'input' || 'select':
        _leaf(node);
        _field(node, form);
      case 'button':
        _button(node, state, surface, depth, form);
      case 'form':
        if (form != null) _fail(node, 'Forms cannot be nested.');
        _require(node, const ['post', 'target']);
        final scope = _Form();
        for (final child in node.children) {
          visual(
            child,
            state,
            surface,
            depth + 1,
            form: scope,
            parent: node.name,
          );
        }
        if (!scope.hasSubmit) _fail(node, 'Form requires a submit button.');
      default:
        for (final child in node.children) {
          visual(
            child,
            state,
            surface,
            depth + 1,
            form: form,
            parent: node.name,
            buttonContent: buttonContent,
          );
        }
    }
  }

  void targets(_Surface surface) {
    for (final target in surface.targets) {
      if (!surface.ids.contains(target.id)) {
        _fail(
          target.node,
          'Target "${target.id}" does not exist in this surface.',
          property: 'target',
        );
      }
    }
  }

  void _action(UiNode node, Map<String, Object?> state, int depth) {
    _visit(node, depth);
    switch (node.name) {
      case 'set':
        _leaf(node);
        if (node.props.isEmpty) _fail(node, 'set requires a state assignment.');
        for (final entry in node.props.entries) {
          if (!state.containsKey(entry.key)) {
            _fail(
              node,
              'Unknown state key "${entry.key}".',
              property: entry.key,
            );
          }
          final initial = state[entry.key];
          final value = _resolve(node, entry.value, state, property: entry.key);
          if (!isStateValue(value) ||
              (initial is num
                  ? value is! num
                  : value.runtimeType != initial.runtimeType)) {
            _fail(
              node,
              'Invalid value type for state key "${entry.key}".',
              property: entry.key,
            );
          }
        }
      case 'popup':
        _properties(node, const {'label': ValueKind.label}, state);
        _require(node, const ['label']);
        if (node.children.length != 1) {
          _fail(node, 'popup requires exactly one content element.');
        }
        final modal = _Surface();
        visual(node.children.single, const {}, modal, depth + 1);
        targets(modal);
      case 'dismiss':
        _properties(node, const {}, state);
        _leaf(node);
      default:
        _fail(node, 'Unsupported action "${node.name}".');
    }
  }

  void _properties(
    UiNode node,
    Map<String, ValueKind> spec,
    Map<String, Object?> state,
  ) {
    for (final entry in node.props.entries) {
      final kind =
          spec[entry.key] ??
          _fail(
            node,
            'Unknown property "${entry.key}" on ${node.name}.',
            property: entry.key,
          );
      final raw = entry.value;
      final value = kind.allowsReference
          ? _resolve(node, raw, state, property: entry.key)
          : raw;
      if (!kind.accepts(value)) {
        _fail(
          node,
          'Property "${entry.key}" expects ${kind.description}.',
          property: entry.key,
        );
      }
    }
  }

  Object? _resolve(
    UiNode node,
    Object? value,
    Map<String, Object?> state, {
    String? property,
  }) {
    if (value is! UiStateRef) return value;
    if (!state.containsKey(value.name)) {
      _fail(node, 'Unknown state key "${value.name}".', property: property);
    }
    final current = state[value.name];
    if (!value.negated) return current;
    if (current is! bool) {
      _fail(node, 'Only Boolean state can be negated.', property: property);
    }
    return !current;
  }

  void _require(UiNode node, List<String> keys) {
    for (final key in keys) {
      if (!node.props.containsKey(key)) {
        _fail(node, '${node.name} requires property "$key".');
      }
    }
  }

  void _leaf(UiNode node) {
    if (node.children.isNotEmpty) {
      _fail(node, '${node.name} cannot contain children.');
    }
  }

  void _visit(UiNode node, int depth) {
    if (depth >= maxUiDepth) {
      _fail(node, 'UI nesting exceeds $maxUiDepth levels.');
    }
    if (++_nodes > maxUiNodes) {
      _fail(node, 'UI has more than $maxUiNodes nodes.');
    }
  }

  Never _fail(UiNode node, String message, {String? property}) {
    final location = node.propertyLocations[property];
    throw UiSyntaxException(
      message,
      location?.line ?? node.line,
      location?.column ?? node.column,
    );
  }
}

final class _Surface {
  final ids = <String>{};
  final targets = <({UiNode node, String id})>[];
}

final class _Form {
  final fields = <String>{};
  bool hasSubmit = false;
}
