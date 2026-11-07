part of 'view.dart';

class _Slot {
  _Slot(UiNode node) {
    install(node);
  }

  late UiNode node;
  late List<_Slot> children;
  Map<String, Object?>? state;
  _FormValues? form;
  bool sending = false;
  bool receiving = false;
  String? error;

  void install(UiNode next) {
    node = next;
    children = next.children.map(_Slot.new).toList(growable: false);
    final initialState = next.props['state'];
    state = initialState is Map<String, Object?> ? Map.of(initialState) : null;
    form = next.name == 'form' ? _FormValues(this) : null;
    sending = receiving = false;
    error = null;
  }

  Iterable<_Slot> get visual sync* {
    yield this;
    for (final child in children) {
      yield* child.visual;
    }
  }

  UiNode snapshot(_Slot? target, UiNode response) => identical(this, target)
      ? response
      : node.withChildren(
          children.map((child) => child.snapshot(target, response)).toList(),
        );
}

String? _symbol(Object? value) => value is UiSymbol ? value.name : null;

Object? _resolve(Object? value, Map<String, Object?> scope) {
  if (value is! UiStateRef) return value;
  final current = scope[value.name];
  return value.negated ? !(current as bool) : current;
}
