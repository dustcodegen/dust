part of 'routing_core.dart';

extension _GeneratedRouterDelegateNavigation<T extends Object>
    on GeneratedRouterDelegate<T> {
  Future<_RouteEntry<T>?> _applyRoute(
    T requested,
    NavigationMode mode, {
    int guardRedirects = 0,
    List<String>? guardRedirectChain,
    bool reuseCachedBranch = true,
  }) async {
    if (_disposed) return null;
    final epoch = ++_navigationEpoch;
    var candidate = requested;
    final guardChain = guardRedirectChain ?? [_debugRoute(requested)];
    _log('${_debugMode(mode)} ${_debugRoute(requested)}');
    _logRouteDetails('route', requested);

    if (guardRedirects >= maxRedirects) {
      throw StateError(
        'Router guard redirects hit the redirect cap ($maxRedirects) '
        'navigating to "${config.routeLocation(requested)}". '
        'Guard redirect chain: ${guardChain.join(' -> ')}. '
        'Check guards that return one of these routes repeatedly.',
      );
    }

    var redirects = 0;
    final redirectChain = <String>[_debugRoute(candidate)];
    for (; redirects < maxRedirects; redirects += 1) {
      final redirected = _routerRedirect(candidate);
      if (redirected == null) break;
      candidate = redirected;
      redirectChain.add(_debugRoute(candidate));
    }
    if (redirects >= maxRedirects) {
      throw StateError(
        'Router hit the redirect cap ($maxRedirects) navigating to '
        '"${config.routeLocation(requested)}". '
        'Redirect chain: ${redirectChain.join(' -> ')}. '
        'Check redirect() for a cycle or return null to allow navigation.',
      );
    }
    if (epoch != _navigationEpoch) return null;

    final targetBranch = config.routeBranch(candidate);
    final retained = reuseCachedBranch
        ? _history.cachedRoutesToKeep(
            candidate,
            targetBranch,
            mode,
            config.routeLocation,
          )
        : null;
    final restored = mode == NavigationMode.restore
        ? config.restoreStack?.call(candidate)
        : null;
    final checkedRoutes = restored == null || restored.isEmpty
        ? <T>[candidate, ...?retained]
        : restored;
    for (final (index, route) in checkedRoutes.indexed) {
      final isCached = retained != null && index > 0;
      if (isCached) {
        final redirected = _routerRedirect(route);
        if (epoch != _navigationEpoch) return null;
        if (redirected != null) {
          return _applyRoute(
            redirected,
            mode,
            guardRedirects: guardRedirects,
            guardRedirectChain: guardChain,
            reuseCachedBranch: false,
          );
        }
      }
      final guardFuture = _runGuards(route);
      final redirected = guardFuture == null ? null : await guardFuture;
      if (epoch != _navigationEpoch) return null;
      if (redirected != null) {
        _log(
          'guard redirect ${_debugRoute(route)} => '
          '${_debugRoute(redirected)}',
        );
        _logRouteDetails('guard target', redirected);
        return _applyRoute(
          redirected,
          mode,
          guardRedirects: guardRedirects + 1,
          guardRedirectChain: [...guardChain, _debugRoute(redirected)],
          // Follow a cached route's redirect without reopening cached pages.
          reuseCachedBranch: reuseCachedBranch && !isCached,
        );
      }
    }
    return _commitRoute(
      candidate,
      mode,
      targetBranch: targetBranch,
      checkedRoutes: checkedRoutes,
      resumesBranch: retained != null,
      reuseCachedBranch: reuseCachedBranch,
    );
  }

  T? _routerRedirect(T route) {
    final redirected = config.router.redirect(route);
    if (redirected == null ||
        config.routeLocation(redirected) == config.routeLocation(route)) {
      return null;
    }
    _log('redirecting ${_debugRoute(route)} => ${_debugRoute(redirected)}');
    _logRouteDetails('redirect target', redirected);
    return redirected;
  }

  Future<T?>? _runGuards(T route) {
    final guards = config.resolveGuards(route);
    if (guards.isEmpty) return null;
    _log('guards ${guards.length} for ${_debugRoute(route)}');
    return _runGuardChain(route, guards);
  }

  Future<T?> _runGuardChain(T route, List<RouteGuardBase<T>> guards) {
    return _runRouteGuards(
      route,
      guards,
      onGuardStart: (guard) {
        _log('guard ${guard.runtimeType} for ${_debugRoute(route)}');
      },
      onGuardAllowed: (guard) {
        _log('guard ${guard.runtimeType} allow ${_debugRoute(route)}');
      },
      onGuardRedirect: (guard, redirected) {
        _log(
          'guard ${guard.runtimeType} redirect '
          '${_debugRoute(route)} => ${_debugRoute(redirected)}',
        );
      },
    );
  }
}
