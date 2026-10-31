import 'package:meta/meta.dart';
import 'package:shelf/shelf.dart';

import 'flatten.dart';
import 'compiled_route.dart';
import 'route.dart';

/// What matching a request against the route table produced.
@internal
sealed class RouteMatch {
  const RouteMatch();
}

/// A route serves this method and path.
@internal
final class Matched extends RouteMatch {
  /// Pairs the handler with what matching produced.
  const Matched(
    this.handler,
    this.parameters, {
    required this.matchedPath,
    this.mountPrefix,
  });

  /// The handler to run.
  final Handler handler;

  /// Path parameters, still percent-encoded.
  final Map<String, String> parameters;

  /// The route pattern that matched, such as `/todos/{id}`.
  final String matchedPath;

  /// The prefix to hand to the mounted handler, when the route is a mount.
  ///
  /// Set only for `mount`, where the inner handler expects paths relative to
  /// where it was mounted rather than absolute ones.
  final String? mountPrefix;
}

/// The path exists, but not for this method.
@internal
final class MethodMismatch extends RouteMatch {
  /// Carries the methods the path does serve.
  const MethodMismatch(this.allowed);

  /// Methods allowed on the path, sorted.
  final List<String> allowed;
}

/// Nothing serves this path.
@internal
final class NoMatch extends RouteMatch {
  /// The only value of its kind.
  const NoMatch();
}

/// Matches requests against a flattened route table.
///
/// Literal prefixes are compiled into a trie once. Each node retains the
/// declaration-ordered routes that can match below that prefix, avoiding a
/// full route-table scan without depending on `shelf_router` internals.
@internal
final class RouteMatcher {
  /// Compiles [routes] into matchable patterns.
  RouteMatcher(List<FlatRoute> routes)
      : _routes = [
          for (var index = 0; index < routes.length; index++)
            CompiledRoute(routes[index], index),
        ] {
    for (final route in _routes) {
      _prefixes.add(route);
    }
    _prefixes.compile(const []);
  }

  final List<CompiledRoute> _routes;
  final _prefixes = _PrefixNode();

  /// The routes that could serve [path], in declaration order.
  ///
  /// A route can only match if the request walked its whole literal prefix.
  /// Each trie node stores that cumulative candidate set in declaration order.
  List<CompiledRoute> _candidates(String path) {
    var node = _prefixes;
    for (final segment in path.split('/')) {
      if (segment.isEmpty) continue;
      final child = node.children[segment];
      if (child == null) break;
      node = child;
    }
    return node.candidates;
  }

  /// Finds the handler for [method] and [path].
  ///
  /// The first route in declaration order whose path matches and whose method
  /// applies wins, which is the rule `shelf_router` uses and the only one that
  /// stays predictable once mounts and any-method routes are in play. A `HEAD`
  /// request with no `HEAD` route falls back to `GET`, as clients expect.
  RouteMatch match(String method, String path) {
    final verb = method.toUpperCase();
    final allowed = <String>{};
    CompiledRoute? getRoute;
    RegExpMatch? getCaptured;

    int? segmentCount;

    for (final route in _candidates(path)) {
      final candidateSegmentCount =
          route.isStatic ? null : segmentCount ??= countSegments(path);
      if (!route.couldMatch(path, candidateSegmentCount)) continue;

      final captured = route.isStatic ? null : route.capture(path);
      if (!route.isStatic && captured == null) continue;

      if (route.method == verb || route.method == Route.anyMethod) {
        return route.matched(captured);
      }

      allowed.add(route.method);
      if (route.method == 'GET') {
        getRoute ??= route;
        getCaptured ??= captured;
      }
    }

    if (verb == 'HEAD' && getRoute != null) {
      return getRoute.matched(getCaptured);
    }
    if (allowed.isEmpty) return const NoMatch();

    return MethodMismatch({
      ...allowed,
      if (allowed.contains('GET')) 'HEAD',
    }.toList()
      ..sort());
  }
}

final class _PrefixNode {
  final children = <String, _PrefixNode>{};
  final _local = <CompiledRoute>[];
  List<CompiledRoute> candidates = const [];

  void add(CompiledRoute route) {
    var node = this;
    for (final segment in route.literalPrefix.split('/')) {
      if (segment.isEmpty) continue;
      node = node.children.putIfAbsent(segment, _PrefixNode.new);
    }
    node._local.add(route);
  }

  void compile(List<CompiledRoute> inherited) {
    candidates = _mergeByDeclaration(inherited, _local);
    for (final child in children.values) {
      child.compile(candidates);
    }
  }
}

List<CompiledRoute> _mergeByDeclaration(
  List<CompiledRoute> inherited,
  List<CompiledRoute> local,
) {
  if (inherited.isEmpty) return List.unmodifiable(local);
  if (local.isEmpty) return inherited;

  final merged = <CompiledRoute>[];
  var inheritedIndex = 0;
  var localIndex = 0;
  while (inheritedIndex < inherited.length && localIndex < local.length) {
    if (inherited[inheritedIndex].index < local[localIndex].index) {
      merged.add(inherited[inheritedIndex++]);
    } else {
      merged.add(local[localIndex++]);
    }
  }
  merged.addAll(inherited.skip(inheritedIndex));
  merged.addAll(local.skip(localIndex));
  return List.unmodifiable(merged);
}
