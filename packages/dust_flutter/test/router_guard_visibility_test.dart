import 'dart:async';

import 'package:dust_flutter/route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/router_runtime_support.dart';

const _private = TestRoute('/private');
const _detail = TestRoute('/detail');
const _login = TestRoute('/login');

void main() {
  testWidgets('a protected initial page is not mounted before guard approval', (
    tester,
  ) async {
    final decision = Completer<TestRoute?>();
    var privateMounts = 0;
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(
        initialRoute: _private,
        requiresAuth: (route) => route.location == _private.location,
        resolveGuards: (route) => [BlockingGuard(decision.future)],
        buildChild: (context, route) => _MountProbe(
          label: route.location,
          onMount: () => privateMounts += 1,
        ),
      ),
    );
    await tester.pumpWidget(MaterialApp.router(routerDelegate: delegate));
    await tester.pumpAndSettle();

    expect(privateMounts, 0);
    expect(find.text('/private'), findsNothing);

    decision.complete(null);
    await tester.pumpAndSettle();

    expect(privateMounts, 1);
    expect(find.text('/private'), findsOneWidget);
  });

  testWidgets('an unguarded initial page is visible on the first frame', (
    tester,
  ) async {
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(buildChild: (context, route) => Text(route.location)),
    );
    await tester.pumpWidget(MaterialApp.router(routerDelegate: delegate));

    expect(find.text('/safe'), findsOneWidget);
  });

  testWidgets('controller pop hides the exposed page and preserves its state', (
    tester,
  ) async {
    final guard = _RecheckGuard();
    var privateMounts = 0;
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(
        resolveGuards: guard.forRoute,
        transitionForRoute: (route) => route.location == _private.location
            ? const BottomToTopPageTransitionsBuilder()
            : null,
        buildChild: (context, route) => route.location == _private.location
            ? _MountProbe(label: '/private', onMount: () => privateMounts += 1)
            : Text(route.location),
      ),
    );
    await pumpRuntimeRouter(tester, delegate);
    final privateResult = delegate.push<String>(_private);
    await tester.pumpAndSettle();
    final privateKey = delegate.debugPageKeys.last;
    final detailResult = delegate.push<String>(_detail);
    await tester.pumpAndSettle();

    guard.decision = Completer<TestRoute?>();
    expect(delegate.pop('detail result'), isTrue);
    await tester.pumpAndSettle();

    expect(find.text('/private'), findsNothing);
    expect(privateMounts, 1);
    await expectLater(detailResult, completion('detail result'));

    guard.decision!.complete(null);
    await tester.pumpAndSettle();

    expect(find.text('/private'), findsOneWidget);
    expect(delegate.debugPageKeys.last, same(privateKey));
    expect(privateMounts, 1);
    expect(delegate.pop('private result'), isTrue);
    await expectLater(privateResult, completion('private result'));
  });

  testWidgets('Navigator removal does not remount a route denied by a guard', (
    tester,
  ) async {
    final guard = _RecheckGuard();
    var privateMounts = 0;
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(
        resolveGuards: guard.forRoute,
        maintainStateForRoute: (route) => route.location != _private.location,
        buildChild: (context, route) => switch (route.location) {
          '/private' => _MountProbe(
              label: '/private',
              onMount: () => privateMounts += 1,
            ),
          '/detail' => TextButton(
              onPressed: () => Navigator.of(context).pop('native result'),
              child: const Text('close detail'),
            ),
          _ => Text(route.location),
        },
      ),
    );
    await pumpRuntimeRouter(tester, delegate);
    final privateResult = delegate.push<String>(_private);
    await tester.pumpAndSettle();
    final detailResult = delegate.push<String>(_detail);
    await tester.pumpAndSettle();
    expect(find.text('/private', skipOffstage: false), findsNothing);

    guard.decision = Completer<TestRoute?>();
    await tester.tap(find.text('close detail'));
    await tester.pumpAndSettle();

    expect(find.text('/private', skipOffstage: false), findsNothing);
    expect(privateMounts, 1);
    await expectLater(detailResult, completion('native result'));

    guard.decision!.complete(_login);
    await tester.pump();
    expect(find.text('/private'), findsNothing);
    await tester.pumpAndSettle();
    expect(find.text('/login'), findsOneWidget);
    expect(privateMounts, 1);
    await expectLater(privateResult, completion(isNull));
  });

  testWidgets('a superseded guard cannot reopen an unchecked page', (
    tester,
  ) async {
    final guard = _RecheckGuard();
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(
        resolveGuards: guard.forRoute,
        buildChild: (context, route) => Text(route.location),
      ),
    );
    await pumpRuntimeRouter(tester, delegate);
    final privateResult = delegate.push<void>(_private);
    await tester.pumpAndSettle();
    delegate.push<void>(_detail);
    await tester.pumpAndSettle();

    final staleDecision = guard.decision = Completer<TestRoute?>();
    expect(delegate.pop(), isTrue);
    await tester.pumpAndSettle();
    delegate.push<void>(_detail);
    await tester.pumpAndSettle();

    final currentDecision = guard.decision = Completer<TestRoute?>();
    expect(await delegate.popRoute(), isTrue);
    staleDecision.complete(null);
    await tester.pumpAndSettle();
    expect(find.text('/private'), findsNothing);

    currentDecision.complete(_login);
    await tester.pumpAndSettle();
    expect(find.text('/login'), findsOneWidget);
    await expectLater(privateResult, completion(isNull));
  });

  testWidgets('a pending page cannot appear in the Navigator Hero overlay', (
    tester,
  ) async {
    final guard = _RecheckGuard();
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(
        resolveGuards: guard.forRoute,
        buildChild: (context, route) => switch (route.location) {
          '/private' => const Center(
              child: Hero(tag: 'shared', child: Text('private hero')),
            ),
          '/detail' => Center(
              child: Column(
                children: [
                  const Hero(tag: 'shared', child: Text('public hero')),
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(),
                    child: const Text('close detail'),
                  ),
                ],
              ),
            ),
          _ => Text(route.location),
        },
      ),
    );
    await pumpRuntimeRouter(tester, delegate);
    delegate.push<void>(_private);
    await tester.pumpAndSettle();
    delegate.push<void>(_detail);
    await tester.pumpAndSettle();

    guard.decision = Completer<TestRoute?>();
    await tester.tap(find.text('close detail'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));
    expect(find.text('private hero'), findsNothing);

    guard.decision!.complete(_login);
    await tester.pumpAndSettle();
    expect(find.text('/login'), findsOneWidget);
  });

  testWidgets('resuming a cached branch shows an approved pending page', (
    tester,
  ) async {
    const branch = TestRoute('/branch');
    const other = TestRoute('/other');
    final guard = _RecheckGuard();
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(
        routeBranch: (route) => switch (route.location) {
          '/branch' || '/private' || '/detail' => 'main',
          '/other' => 'other',
          _ => null,
        },
        resolveGuards: guard.forRoute,
        buildChild: (context, route) => Text(route.location),
      ),
    );
    await pumpRuntimeRouter(tester, delegate);
    delegate.go(branch);
    await tester.pumpAndSettle();
    final privateResult = delegate.push<String>(_private);
    await tester.pumpAndSettle();
    delegate.push<void>(_detail);
    await tester.pumpAndSettle();

    final staleDecision = guard.decision = Completer<TestRoute?>();
    expect(delegate.pop(), isTrue);
    delegate.go(other);
    staleDecision.complete(null);
    await tester.pumpAndSettle();

    guard.decision = Completer<TestRoute?>();
    delegate.go(branch);
    await tester.pumpAndSettle();
    expect(find.text('/other'), findsOneWidget);
    expect(find.text('/private'), findsNothing);

    guard.decision!.complete(null);
    await tester.pumpAndSettle();
    expect(find.text('/private'), findsOneWidget);
    expect(delegate.pop('restored result'), isTrue);
    await expectLater(privateResult, completion('restored result'));
  });
}

final class _RecheckGuard implements AsyncRouteGuard<TestRoute> {
  Completer<TestRoute?>? decision;

  List<RouteGuardBase<TestRoute>> forRoute(TestRoute route) =>
      route.location == _private.location ? [this] : const [];

  @override
  Future<TestRoute?> canActivate(TestRoute route) async =>
      await decision?.future;
}

final class _MountProbe extends StatefulWidget {
  const _MountProbe({required this.label, required this.onMount});

  final String label;
  final VoidCallback onMount;

  @override
  State<_MountProbe> createState() => _MountProbeState();
}

final class _MountProbeState extends State<_MountProbe> {
  @override
  void initState() {
    super.initState();
    widget.onMount();
  }

  @override
  Widget build(BuildContext context) => Text(widget.label);
}
