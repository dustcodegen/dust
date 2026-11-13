import 'dart:async';

import 'package:dust_flutter/route.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/router_runtime_support.dart';

void main() {
  test(
    'refresh redirect reuses the previous page and its pending result',
    () async {
      final router = RefreshRouter();
      addTearDown(router.refreshNotifier.dispose);
      final decision = Completer<TestRoute?>();
      var redirectOnRefresh = false;
      final delegate = GeneratedRouterDelegate<TestRoute>(
        runtimeConfig(
          router: router,
          resolveGuards: (route) =>
              redirectOnRefresh && route.location == '/login'
              ? [BlockingGuard(decision.future)]
              : const [],
        ),
      );
      addTearDown(delegate.dispose);
      await delegate.debugWaitForScheduledRefresh();

      final products = delegate.push<String>(const TestRoute('/products'));
      final productsKey = delegate.debugPageKeys.last;
      final login = delegate.push<bool>(const TestRoute('/login'));

      redirectOnRefresh = true;
      router.refreshNotifier.notifyListeners();
      final refresh = delegate.debugWaitForScheduledRefresh();
      final destination = TestRoute('/products');
      decision.complete(destination);
      await refresh;

      expect(locations(delegate.stack), '[/safe, /products]');
      expect(delegate.currentRoute, same(destination));
      expect(delegate.debugPageKeys.last, same(productsKey));
      await expectLater(login, completion(isNull));

      expect(delegate.pop('selected product'), isTrue);
      await expectLater(products, completion('selected product'));
    },
  );

  test(
    'explicit replace and ordinary refresh preserve duplicate pages',
    () async {
      final router = RefreshRouter();
      addTearDown(router.refreshNotifier.dispose);
      final delegate = GeneratedRouterDelegate<TestRoute>(
        runtimeConfig(router: router),
      );
      addTearDown(delegate.dispose);
      await delegate.debugWaitForScheduledRefresh();

      final detail = delegate.push<void>(const TestRoute('/detail'));
      delegate.replace(const TestRoute('/safe'));
      await expectLater(detail, completion(isNull));

      expect(locations(delegate.stack), '[/safe, /safe]');
      final keys = delegate.debugPageKeys;
      expect(keys.first, isNot(same(keys.last)));

      router.refreshNotifier.notifyListeners();
      await delegate.debugWaitForScheduledRefresh();

      expect(locations(delegate.stack), '[/safe, /safe]');
      expect(delegate.debugPageKeys, keys);
    },
  );
}
