part of 'view.dart';

class _Layout {
  _Layout(this.view);

  final _DustUiViewState view;

  Widget build(
    _Slot slot,
    Map<String, Object?> inherited,
    _FormValues? form, {
    bool active = true,
  }) {
    final node = slot.node;
    final scope = slot.state ?? inherited;
    final children = slot.children;
    final align = _symbol(node.props['align']);
    final gap = (node.props['gap'] as num?)?.toDouble() ?? 0;
    final visible = _visible(slot, inherited);
    active = active && visible;
    final Widget child;
    switch (node.name) {
      case 'col':
        child = Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: switch (align) {
            'start' => CrossAxisAlignment.start,
            'center' => CrossAxisAlignment.center,
            'end' => CrossAxisAlignment.end,
            _ => CrossAxisAlignment.stretch,
          },
          children: _linear(
            children,
            scope,
            form,
            gap,
            active,
            horizontal: false,
          ),
        );
      case 'row':
        child = Row(
          crossAxisAlignment: switch (align) {
            'start' => CrossAxisAlignment.start,
            'end' => CrossAxisAlignment.end,
            _ => CrossAxisAlignment.center,
          },
          children: _linear(
            children,
            scope,
            form,
            gap,
            active,
            horizontal: true,
          ),
        );
      case 'zstack':
        child = Stack(
          alignment: switch (align) {
            'start' => AlignmentDirectional.centerStart,
            'end' => AlignmentDirectional.centerEnd,
            'bottomEnd' => AlignmentDirectional.bottomEnd,
            _ => Alignment.center,
          },
          children: [
            for (final child in children)
              build(child, scope, form, active: active),
          ],
        );
      case 'text':
        child = Text(
          _resolve(node.props['value'], scope)! as String,
          style: TextStyle(
            fontSize: (node.props['size'] as num?)?.toDouble(),
            fontWeight: _symbol(node.props['weight']) == 'bold'
                ? FontWeight.bold
                : null,
          ),
        );
      case 'button':
        child = _button(slot, scope, form, active);
      case 'form':
        final values = slot.form!;
        child = Form(
          key: values.key,
          child: _withError(
            Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                for (final child in children)
                  build(child, scope, values, active: active),
              ],
            ),
            slot.error,
          ),
        );
      case 'input':
      case 'select':
        child = form!.field(node, scope, active: active);
      default:
        throw StateError('Unsupported visual element: ${node.name}');
    }
    final padding = (node.props['padding'] as num?)?.toDouble() ?? 0;
    return KeyedSubtree(
      key: ObjectKey(node),
      child: Visibility(
        visible: visible,
        maintainState: true,
        child: ExcludeFocus(
          excluding: !visible,
          child: padding == 0
              ? child
              : Padding(padding: EdgeInsets.all(padding), child: child),
        ),
      ),
    );
  }

  bool _visible(_Slot slot, Map<String, Object?> inherited) =>
      _resolve(slot.node.props['visible'], slot.state ?? inherited) != false;

  List<Widget> _linear(
    List<_Slot> children,
    Map<String, Object?> scope,
    _FormValues? form,
    double gap,
    bool active, {
    required bool horizontal,
  }) {
    var spaced = false;
    final widgets = <Widget>[];
    for (final slot in children) {
      final key = ObjectKey(slot.node);
      if (slot.node.name == 'spacer') {
        widgets.add(Spacer(key: key));
        continue;
      }
      final visible =
          _visible(slot, scope) && _symbol(slot.node.props['type']) != 'hidden';
      final space = visible && spaced ? gap : 0.0;
      spaced = spaced || visible;
      final child = Padding(
        key: key,
        padding: EdgeInsetsDirectional.only(
          start: horizontal ? space : 0,
          top: horizontal ? 0 : space,
        ),
        child: build(slot, scope, form, active: active),
      );
      widgets.add(horizontal ? Flexible(key: key, child: child) : child);
    }
    return widgets;
  }

  Widget _button(
    _Slot slot,
    Map<String, Object?> scope,
    _FormValues? form,
    bool active,
  ) {
    final node = slot.node;
    final submit = _symbol(node.props['type']) == 'submit';
    final action = (node.props['onTap'] as UiActionBlock?)?.actions.single;
    final origin = submit ? form!.slot : slot;
    final busy = view._busy(origin);
    final disabled =
        !active ||
        _resolve(node.props['disabled'], scope) == true ||
        (action?.name == 'dismiss' && view.widget._dismiss == null);
    return _withError(
      Semantics(
        expanded: _resolve(node.props['expanded'], scope) as bool?,
        child: FilledButton(
          onPressed: busy || disabled
              ? null
              : () {
                  if (submit) {
                    final body = form!.capture();
                    if (body != null) view._send(origin, body);
                  } else if (action != null) {
                    view._action(action, scope);
                  } else {
                    view._send(origin, const {}, _buttonLabel(slot, scope));
                  }
                },
          child: Semantics(
            label: origin.sending
                ? null
                : node.props['accessibilityLabel'] as String?,
            excludeSemantics:
                !origin.sending && node.props.containsKey('accessibilityLabel'),
            liveRegion: origin.sending,
            child: origin.sending
                ? const Text('Working…')
                : build(slot.children.single, scope, form, active: active),
          ),
        ),
      ),
      slot.error,
    );
  }

  String _buttonLabel(_Slot slot, Map<String, Object?> scope) {
    final explicit = slot.node.props['accessibilityLabel'] as String?;
    if (explicit != null) return explicit;
    final text = <String>[];
    void collect(_Slot content) {
      if (!_visible(content, scope)) return;
      if (content.node.name == 'text') {
        text.add(_resolve(content.node.props['value'], scope)! as String);
      }
      for (final child in content.children) {
        collect(child);
      }
    }

    collect(slot);
    return text.isEmpty ? 'Dialog' : text.join(' ');
  }

  Widget _withError(Widget child, String? error) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        child,
        if (error != null) Semantics(liveRegion: true, child: Text(error)),
      ],
    );
  }
}
