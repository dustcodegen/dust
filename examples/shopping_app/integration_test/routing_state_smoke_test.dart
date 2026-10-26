import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:shopping_app/core/view_models/app_view_model.dart';
import 'package:shopping_app/route.dart';

import '../test/support/routing_support.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('generated routing and state run on a Flutter platform', (
    tester,
  ) async {
    final router = await shoppingRouter();
    final delegate = GeneratedRouterDelegate<ShoppingRoute>(
      RouterRuntimeConfig<ShoppingRoute>(
        router: router,
        initialRoute: const ProductsRoute(),
        parseRoute: parseShoppingRoute,
        routeLocation: shoppingRouteLocation,
        requiresAuth: shoppingRouteRequiresAuth,
        routeBranch: shoppingRouteBranch,
        resolveGuards: (route) => shoppingRouteGuards(route, router),
        restoreStack: restoreShoppingRouteStack,
        buildPage: (route, key, onPopInvoked) => generatedPage<void>(
          key: key,
          location: route.location,
          name: route.location,
          onPopInvoked: onPopInvoked,
          child: _SmokePage(route: route),
        ),
      ),
    );

    await tester.pumpWidget(
      BnbViewModelScope(
        args: (_) => const BnbViewModelArgs(),
        create: (_, args) => BnbViewModel(args),
        child: MaterialApp.router(
          routerConfig: RouterConfig<ShoppingRoute>(
            routeInformationProvider: PlatformRouteInformationProvider(
              initialRouteInformation: RouteInformation(
                uri: Uri.parse('/product/7?campaign=device#reviews'),
              ),
            ),
            routeInformationParser:
                GeneratedRouteInformationParser<ShoppingRoute>(
              router: router,
              parseRoute: parseShoppingRoute,
              routeLocation: shoppingRouteLocation,
            ),
            routerDelegate: delegate,
            backButtonDispatcher: RootBackButtonDispatcher(),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('/product/7?campaign=device#reviews'), findsOneWidget);
    expect(find.text('tab:home'), findsOneWidget);

    await tester.tap(find.text('select products'));
    await tester.pump();
    expect(find.text('tab:products'), findsOneWidget);

    delegate.go(const AdminRoute());
    await tester.pumpAndSettle();
    expect(find.text('/login?redirectPath=%2Fadmin'), findsOneWidget);

    authenticate(router, 'dust');
    await tester.pumpAndSettle();
    expect(find.text('/'), findsOneWidget);
  });
}

final class _SmokePage extends StatelessWidget {
  const _SmokePage({required this.route});

  final ShoppingRoute route;

  @override
  Widget build(BuildContext context) {
    final tab = context.watchBnbViewModel().value.currentTab;
    return Scaffold(
      body: Column(
        children: [
          Text(route.location),
          Text('tab:${tab.name}'),
          TextButton(
            onPressed: () => context.readBnbViewModel().select(BnbTab.products),
            child: const Text('select products'),
          ),
        ],
      ),
    );
  }
}
