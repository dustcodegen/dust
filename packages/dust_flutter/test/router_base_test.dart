import 'package:dust_flutter/route.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('router base defaults are inert and preserve route information', () {
    final router = _Router();
    final information = RouteInformation(uri: Uri.parse('/products?page=2'));
    final config = RouterRuntimeConfig<_Route>(
      router: router,
      initialRoute: const _Route('/'),
      parseRoute: (uri) => _Route(uri.toString()),
      routeLocation: (route) => route.location,
      requiresAuth: (_) => false,
      resolveGuards: (_) => const [],
      buildPage: (_, key, onPopInvoked) => _Page(
        key: key,
        onPopInvoked: onPopInvoked,
      ),
    );

    expect(router.refreshListenable, isNull);
    expect(router.observers, isEmpty);
    expect(router.debugLogDiagnostics, isFalse);
    expect(router.parseRouteInformation(information), same(information));
    expect(router.redirect(const _Route('/')), isNull);
    expect(config.routeBranch(const _Route('/')), isNull);
    router.didChangeRouteStack(const [_Route('/')], const [_Route('/next')]);
  });

  test('default exception hook reports Flutter routing context', () {
    final router = _Router();
    final previousOnError = FlutterError.onError;
    late FlutterErrorDetails reported;
    FlutterError.onError = (details) => reported = details;
    addTearDown(() => FlutterError.onError = previousOnError);
    final error = StateError('failed');
    final stackTrace = StackTrace.current;

    router.onException(error, stackTrace);

    expect(reported.exception, same(error));
    expect(reported.stack, same(stackTrace));
    expect(reported.library, 'dust_flutter routing');
    expect(reported.context.toString(), contains('generated router'));
  });
}

final class _Route {
  const _Route(this.location);

  final String location;
}

final class _Router extends RouterBase<_Route> {}

final class _Page extends Page<void> {
  const _Page({required super.key, required super.onPopInvoked});

  @override
  Route<void> createRoute(BuildContext context) => PageRouteBuilder<void>(
        pageBuilder: (context, animation, secondaryAnimation) =>
            const SizedBox(),
      );
}
