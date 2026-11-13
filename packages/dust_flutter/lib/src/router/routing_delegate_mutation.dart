part of 'routing_core.dart';

extension _GeneratedRouterDelegateMutation<T extends Object>
    on GeneratedRouterDelegate<T> {
  _RouteEntry<T> _commitRoute(
    T route,
    NavigationMode mode, {
    required String? targetBranch,
    required RouteStack<T> checkedRoutes,
    required bool resumesBranch,
    required bool reuseCachedBranch,
    required bool refreshing,
  }) {
    final previous = _history.routes();
    final previousBranch = _history.activeBranch;
    final switchedBranch = _history.activateBranch(targetBranch);
    if (switchedBranch) {
      _log('branch ${previousBranch ?? '-'} => ${targetBranch ?? '-'}');
      if (!reuseCachedBranch) {
        _completeEntries(_history.entries);
        _history.entries.clear();
      }
    }
    late final _RouteEntry<T> committed;
    switch (mode) {
      case NavigationMode.go:
        if (resumesBranch) {
          _history.entries[0] = _history.entries.first.withRoute(route);
          committed = _history.entries.last;
          break;
        }
        committed = _RouteEntry(route);
        _completeEntries(_history.replaceWithOne(committed));
      case NavigationMode.push:
        committed = _RouteEntry(route, tracksPop: true);
        _history.push(committed);
      case NavigationMode.replace:
        final replacement = _history.replaceTop(
          route,
          config.routeLocation,
          reusePrevious: refreshing && !switchedBranch,
        );
        committed = replacement.committed;
        replacement.replaced?.complete(null);
      case NavigationMode.restore:
        _completeEntries(_history.replaceWithRoutes(checkedRoutes));
        committed = _history.entries.last;
    }
    // Cached entries can have a pending check from an earlier, superseded pop.
    for (final entry in resumesBranch ? _history.entries : [committed]) {
      entry.key.readiness.value = _RoutePageReadiness.ready;
    }
    _finishStackCommit(previous);
    return committed;
  }

  void _completeEntries(Iterable<_RouteEntry<T>> entries) {
    for (final entry in entries) {
      entry.complete(null);
    }
  }

  bool _popTop(Object? result) {
    if (_disposed) return false;
    if (_history.entries.length <= 1) return false;
    final previous = _history.routes();
    final removed = _history.entries.removeLast();
    removed.complete(result);
    _log('pop ${_debugRoute(removed.route)}');
    _finishStackRemoval(previous, exposesTop: true);
    return true;
  }

  void _finishStackRemoval(RouteStack<T> previous, {required bool exposesTop}) {
    if (exposesTop) _history.entries.lastOrNull?.key.waitForCheck();
    _finishStackCommit(previous);
    _revalidateExposedTop();
  }

  void _revalidateExposedTop() {
    if (_disposed || _history.entries.isEmpty) return;
    _unawaitedNavigation(_applyRoute(currentRoute, NavigationMode.replace));
  }

  void _unawaitedNavigation(Future<void> future) {
    unawaited(
      future.catchError((Object error, StackTrace stackTrace) {
        config.router.onException(error, stackTrace);
      }),
    );
  }
}
