part of 'validator.dart';

extension _Controls on _Validator {
  void _button(
    UiNode node,
    Map<String, Object?> state,
    _Surface surface,
    int depth,
    _Form? form,
  ) {
    final hasGet = node.props.containsKey('get');
    final hasPost = node.props.containsKey('post');
    final hasType = node.props.containsKey('type');
    final hasTap = node.props.containsKey('onTap');
    final behaviors = [hasGet, hasPost, hasType, hasTap];
    if (behaviors.where((value) => value).length != 1) {
      _fail(node, 'Button requires exactly one of get, post, type, or onTap.');
    }
    final hasTarget = node.props.containsKey('target');
    final hasOpen = node.props.containsKey('open');
    if (hasGet || hasPost) {
      if (hasTarget == hasOpen) {
        _fail(node, 'HTTP button requires exactly one of target or open.');
      }
    } else if (hasTarget || hasOpen) {
      _fail(node, 'Only HTTP buttons support target or open.');
    }
    if (hasType) {
      if (form == null) _fail(node, 'Submit button must be inside a form.');
      form.hasSubmit = true;
    }
    if (hasTap) {
      final block = node.props['onTap']! as UiActionBlock;
      if (block.actions.length != 1) {
        _fail(node, 'onTap requires exactly one action.', property: 'onTap');
      }
      _action(block.actions.single, state, depth + 1);
    }
    if (node.children.length != 1) {
      _fail(node, 'Button requires exactly one visual child.');
    }
    visual(
      node.children.single,
      state,
      surface,
      depth + 1,
      parent: node.name,
      buttonContent: true,
    );
  }

  void _field(UiNode node, _Form? form) {
    if (form == null) _fail(node, '${node.name} must be inside a form.');
    _require(node, const ['name']);
    final name = node.props['name']! as String;
    if (!form.fields.add(name)) _fail(node, 'Duplicate form field "$name".');

    if (node.name == 'select') {
      _require(node, const ['label', 'options']);
      final options = node.props['options']! as List<Object?>;
      if (node.props.containsKey('initial') &&
          !options.contains(node.props['initial'])) {
        _fail(node, 'Initial selection must be one of the options.');
      }
      return;
    }
    if (node.props.containsKey('type')) {
      _require(node, const ['value']);
      const allowed = {'name', 'type', 'value'};
      if (node.props.keys.any((key) => !allowed.contains(key))) {
        _fail(node, 'Hidden input supports only name, type, and value.');
      }
    } else {
      _require(node, const ['label']);
      if (node.props.containsKey('value')) {
        _fail(node, 'Editable input uses initial instead of value.');
      }
    }
  }
}
