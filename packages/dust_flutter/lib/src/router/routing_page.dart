part of 'routing_core.dart';

/// Builds a generated Flutter page for one typed route.
Page<R> generatedPage<R>({
  required LocalKey key,
  required String location,
  required String name,
  required Widget child,
  PopInvokedWithResultCallback<Object?>? onPopInvoked,
  PageTransitionsBuilder? transition,
  bool fullscreenDialog = false,
  bool maintainState = true,
}) {
  final content = key is _RoutePageKey
      ? _RoutePageContent(pageKey: key, child: child)
      : child;
  if (transition != null) {
    return _GeneratedTransitionPage<R>(
      key: key,
      name: name,
      onPopInvoked: (didPop, result) => onPopInvoked?.call(didPop, result),
      fullscreenDialog: fullscreenDialog,
      maintainState: maintainState,
      transition: transition,
      child: content,
    );
  }

  return MaterialPage<R>(
    key: key,
    name: name,
    onPopInvoked: (didPop, result) => onPopInvoked?.call(didPop, result),
    fullscreenDialog: fullscreenDialog,
    maintainState: maintainState,
    child: content,
  );
}

enum _RoutePageReadiness { initial, waiting, ready }

final class _RoutePageKey extends UniqueKey {
  final ValueNotifier<_RoutePageReadiness> readiness = ValueNotifier(
    _RoutePageReadiness.ready,
  );

  void waitForCheck() {
    if (readiness.value == _RoutePageReadiness.ready) {
      readiness.value = _RoutePageReadiness.waiting;
    }
  }
}

final class _PendingRoutePage extends MaterialPage<Object?> {
  const _PendingRoutePage({required super.key})
    : super(child: const SizedBox.shrink());
}

final class _RoutePageContent extends StatefulWidget {
  const _RoutePageContent({required this.pageKey, required this.child});

  final _RoutePageKey pageKey;
  final Widget child;

  @override
  State<_RoutePageContent> createState() => _RoutePageContentState();
}

final class _RoutePageContentState extends State<_RoutePageContent> {
  bool _hasShownContent = false;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder(
      valueListenable: widget.pageKey.readiness,
      builder: (context, readiness, child) {
        final ready = readiness == _RoutePageReadiness.ready;
        _hasShownContent = _hasShownContent || ready;
        // Keep existing state, but defer mounting content dropped by Navigator.
        return HeroMode(
          enabled: ready,
          child: Visibility(
            visible: ready,
            maintainState: true,
            child: _hasShownContent ? widget.child : const SizedBox.shrink(),
          ),
        );
      },
    );
  }
}

final class _GeneratedTransitionPage<R> extends Page<R> {
  const _GeneratedTransitionPage({
    required this.child,
    required this.transition,
    required this.fullscreenDialog,
    required this.maintainState,
    required super.key,
    required super.name,
    required super.onPopInvoked,
  });

  final Widget child;
  final PageTransitionsBuilder transition;
  final bool fullscreenDialog;
  final bool maintainState;

  @override
  Route<R> createRoute(BuildContext context) {
    return _GeneratedTransitionRoute<R>(page: this);
  }
}

final class _GeneratedTransitionRoute<R> extends PageRoute<R> {
  _GeneratedTransitionRoute({
    required _GeneratedTransitionPage<R> page,
  }) : super(settings: page);

  _GeneratedTransitionPage<R> get _page =>
      settings as _GeneratedTransitionPage<R>;

  @override
  bool get fullscreenDialog => _page.fullscreenDialog;

  @override
  bool get maintainState => _page.maintainState;

  @override
  Duration get transitionDuration => _page.transition.transitionDuration;

  @override
  Duration get reverseTransitionDuration =>
      _page.transition.reverseTransitionDuration;

  @override
  Color? get barrierColor => null;

  @override
  String? get barrierLabel => null;

  @override
  DelegatedTransitionBuilder? get delegatedTransition =>
      _page.transition.delegatedTransition;

  @override
  Widget buildPage(
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
  ) {
    return Semantics(
      scopesRoute: true,
      explicitChildNodes: true,
      child: _page.child,
    );
  }

  @override
  Widget buildTransitions(
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    return _page.transition.buildTransitions<R>(
      this,
      context,
      animation,
      secondaryAnimation,
      child,
    );
  }
}

/// Bottom-to-top page transition for fullscreen dialog routes.
class BottomToTopPageTransitionsBuilder extends PageTransitionsBuilder {
  /// Creates a bottom-to-top page transition builder.
  const BottomToTopPageTransitionsBuilder();

  @override
  Widget buildTransitions<T>(
    PageRoute<T> route,
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    if (MediaQuery.of(context).disableAnimations) return child;
    return SlideTransition(
      position: Tween<Offset>(
        begin: const Offset(0, 1),
        end: Offset.zero,
      ).animate(CurvedAnimation(parent: animation, curve: Curves.easeOutCubic)),
      child: child,
    );
  }
}
