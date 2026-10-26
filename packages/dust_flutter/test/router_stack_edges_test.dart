import 'package:dust_flutter/route.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/router_runtime_support.dart';

void main() {
  test('switching from a branch restores the root stack', () async {
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(
        routeBranch: (route) =>
            route.location.startsWith('/tab') ? 'mainTabs' : null,
      ),
    );
    await delegate.debugWaitForScheduledRefresh();

    delegate.go(const TestRoute('/tab'));
    await Future<void>.delayed(Duration.zero);
    delegate.go(const TestRoute('/safe'));
    await Future<void>.delayed(Duration.zero);

    expect(delegate.stack.map((route) => route.location), ['/safe']);
  });

  test('replacing into a new branch creates its first route', () async {
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(
        routeBranch: (route) =>
            route.location.startsWith('/tab') ? 'mainTabs' : null,
      ),
    );
    await delegate.debugWaitForScheduledRefresh();

    delegate.replace(const TestRoute('/tab'));
    await Future<void>.delayed(Duration.zero);

    expect(delegate.stack.map((route) => route.location), ['/tab']);
  });
}
