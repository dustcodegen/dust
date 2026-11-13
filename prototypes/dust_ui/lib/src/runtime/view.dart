/// Flutter host for one validated Dust UI surface.
library;

import 'package:flutter/material.dart';

import '../../language.dart';

part 'fields.dart';
part 'layout.dart';
part 'slots.dart';

/// A request described by an interactive element.
class UiRequest {
  /// Creates an immutable request for the host transport.
  UiRequest(this.method, this.path, Map<String, Object?> body)
    : body = Map.unmodifiable(body);

  /// The GET or POST method.
  final String method;

  /// The root-relative destination within the host's configured origin.
  final String path;

  /// The current form values, if this is a form submission.
  final Map<String, Object?> body;
}

/// Renders one validated document. The host owns HTTP and response validation.
class DustUiView extends StatefulWidget {
  /// Mounts [document], using [request] to fetch validated response trees.
  const DustUiView({required this.document, required this.request, super.key})
    : _dismiss = null;

  const DustUiView._popup(
    this._dismiss, {
    required this.document,
    required this.request,
  });

  /// The validated tree for this surface.
  final UiNode document;

  /// Host transport and parser for UI requests.
  final Future<UiNode> Function(UiRequest) request;
  final VoidCallback? _dismiss;

  @override
  State<DustUiView> createState() => _DustUiViewState();
}

class _DustUiViewState extends State<DustUiView> {
  late _Slot _root = _Slot(widget.document);
  DialogRoute<void>? _dialog;

  @override
  void didUpdateWidget(covariant DustUiView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(widget.document, oldWidget.document)) {
      _root = _Slot(widget.document);
      _discardDialog();
    }
  }

  @override
  void dispose() {
    _discardDialog();
    super.dispose();
  }

  void _discardDialog() {
    final route = _dialog;
    _dialog = null;
    if (route != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (route.isActive) route.navigator?.removeRoute(route);
      });
    }
  }

  @override
  Widget build(BuildContext context) =>
      _Layout(this).build(_root, const {}, null);

  _Slot? _target(_Slot origin) {
    final id = origin.node.props['target'] as String?;
    if (id == null) return null;
    for (final slot in _root.visual) {
      if (slot.node.props['id'] == id) return slot;
    }
    return null;
  }

  bool _busy(_Slot origin) =>
      origin.sending || (_target(origin)?.receiving ?? false);

  Future<void> _send(
    _Slot origin, [
    Map<String, Object?> body = const {},
    String? popupLabel,
  ]) async {
    if (_busy(origin)) return;
    final node = origin.node;
    final open = _symbol(node.props['open']) == 'popup';
    final target = _target(origin);
    if (!open && target == null) {
      setState(
        () => origin.error = 'This action’s destination is unavailable.',
      );
      return;
    }
    final targetNode = target?.node;
    final document = widget.document;
    bool current() =>
        mounted &&
        identical(widget.document, document) &&
        identical(origin.node, node) &&
        _root.visual.contains(origin);
    bool currentTarget() =>
        target != null &&
        identical(target.node, targetNode) &&
        _root.visual.contains(target);

    setState(() {
      origin
        ..sending = true
        ..error = null;
      if (target != null) target.receiving = true;
    });
    try {
      final get = node.props['get'] as String?;
      final response = await widget.request(
        UiRequest(
          get == null ? 'POST' : 'GET',
          get ?? node.props['post']! as String,
          body,
        ),
      );
      if (!current()) return;
      if (open) {
        await _open(response, popupLabel);
      } else if (currentTarget()) {
        if (response.props['id'] != targetNode!.props['id']) {
          throw const FormatException('Response does not match the target.');
        }
        validateUi(_root.snapshot(target, response));
        setState(() => target!.install(response));
      }
    } catch (_) {
      if (current()) {
        setState(() => origin.error = 'Could not complete the request.');
      }
    } finally {
      if (current() || (mounted && currentTarget())) {
        setState(() {
          if (current()) origin.sending = false;
          if (currentTarget()) target!.receiving = false;
        });
      }
    }
  }

  void _action(UiNode action, Map<String, Object?> scope) {
    switch (action.name) {
      case 'set':
        final updates = action.props.map(
          (key, value) => MapEntry(key, _resolve(value, scope)),
        );
        setState(() => scope.addAll(updates));
      case 'popup':
        _open(action.children.single, action.props['label'] as String?);
      case 'dismiss':
        widget._dismiss?.call();
    }
  }

  Future<void> _open(UiNode document, String? label) async {
    if (_dialog != null) return;
    validateUi(document);
    final navigator = Navigator.of(context, rootNavigator: true);
    final route = DialogRoute<void>(
      context: context,
      themes: InheritedTheme.capture(from: context, to: navigator.context),
      requestFocus: true,
      traversalEdgeBehavior: TraversalEdgeBehavior.closedLoop,
      builder: (context) {
        void dismiss() => Navigator.of(context).pop();
        return AlertDialog(
          semanticLabel: label,
          scrollable: true,
          content: SizedBox(
            width: 440,
            child: DustUiView._popup(
              dismiss,
              document: document,
              request: widget.request,
            ),
          ),
          actions: [
            IconButton(
              tooltip: 'Close dialog',
              onPressed: dismiss,
              icon: const Icon(Icons.close),
            ),
          ],
        );
      },
    );
    _dialog = route;
    try {
      await navigator.push(route);
    } finally {
      if (identical(_dialog, route)) _dialog = null;
    }
  }
}
