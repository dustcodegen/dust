import 'package:dust_flutter/route.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('route annotations retain every declared option', () {
    final initial = '/';
    final router = AppRouter(initial: initial, notFound: '/404');
    final transition = BottomToTopPageTransitionsBuilder();
    final route = AppRoute(
      '/detail',
      name: 'detail',
      result: bool,
      shell: _Shell,
      branch: 'main',
      guards: const [_Guard],
      transition: transition,
      fullscreenDialog: true,
      maintainState: false,
    );

    expect(router.initial, '/');
    expect(router.notFound, '/404');
    expect(route.path, '/detail');
    expect(route.name, 'detail');
    expect(route.result, bool);
    expect(route.shell, _Shell);
    expect(route.branch, 'main');
    expect(route.guards, const [_Guard]);
    expect(route.transition, same(transition));
    expect(route.fullscreenDialog, isTrue);
    expect(route.maintainState, isFalse);
  });

  test('generated route metadata traverses and finds nested routes', () {
    final child = GeneratedRoute(
      'detail',
      page: _DetailPage,
      name: 'detail',
    );
    final root = GeneratedRoute(
      '/',
      page: _HomePage,
      name: 'home',
      routes: [child],
    );

    expect(root.depthFirst, [root, child]);
    expect(root.findByName('detail'), same(child));
    expect(root.findByName('missing'), isNull);
    expect(root.findByPage(_DetailPage), same(child));
    expect(root.findByPage(_MissingPage), isNull);
  });
}

final class _Guard {}

final class _Shell {}

final class _HomePage {}

final class _DetailPage {}

final class _MissingPage {}
