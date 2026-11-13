import 'dart:async';

import 'package:dust_flutter/route.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/router_runtime_support.dart';

const _home = TestRoute('/home');
const _detail = TestRoute('/home/detail');
const _confirmation = TestRoute('/home/detail/confirmation');
const _other = TestRoute('/other');
const _otherDetail = TestRoute('/other/detail');

String? _branch(TestRoute route) {
  if (route.location.startsWith('/home')) return 'home';
  if (route.location.startsWith('/other')) return 'other';
  return null;
}

void main() {
  test(
    'branch resume checks all routes and keeps existing push entries',
    () async {
      final calls = <String>[];
      final delegate = GeneratedRouterDelegate<TestRoute>(
        runtimeConfig(
          routeBranch: _branch,
          resolveGuards: (route) => [RecordingGuard(route.location, calls)],
        ),
      );
      addTearDown(delegate.dispose);
      await delegate.debugWaitForScheduledRefresh();

      await delegate.setNewRoutePath(_home);
      final detailResult = delegate.push<String>(_detail);
      await Future<void>.delayed(Duration.zero);
      final confirmationResult = delegate.push<int>(_confirmation);
      await Future<void>.delayed(Duration.zero);
      final keys = delegate.debugPageKeys;
      delegate.go(_other);
      await Future<void>.delayed(Duration.zero);

      calls.clear();
      final requestedRoot = TestRoute(_home.location);
      delegate.go(requestedRoot);
      await Future<void>.delayed(Duration.zero);

      expect(calls, [_home.location, _detail.location, _confirmation.location]);
      expect(delegate.stack.first, same(requestedRoot));
      expect(delegate.debugPageKeys, keys);
      expect(delegate.pop(42), isTrue);
      expect(await confirmationResult, 42);
      await Future<void>.delayed(Duration.zero);
      expect(delegate.pop('saved'), isTrue);
      expect(await detailResult, 'saved');
    },
  );

  test(
    'an async guard can send a cached branch back to its public root',
    () async {
      final check = Completer<TestRoute?>();
      final calls = <String>[];
      var rechecking = false;
      final router = RecordingRouter();
      final delegate = GeneratedRouterDelegate<TestRoute>(
        runtimeConfig(
          router: router,
          routeBranch: _branch,
          resolveGuards: (route) => switch (route.location) {
            '/home/detail' when rechecking => [BlockingGuard(check.future)],
            '/home/detail/confirmation' => [
              RecordingGuard(route.location, calls),
            ],
            _ => const [],
          },
        ),
      );
      addTearDown(delegate.dispose);
      await delegate.debugWaitForScheduledRefresh();

      await delegate.setNewRoutePath(_home);
      final detailResult = delegate.push<String>(_detail);
      final confirmationResult = delegate.push<int>(_confirmation);
      await Future<void>.delayed(Duration.zero);
      delegate.go(_other);

      rechecking = true;
      calls.clear();
      router.stackChanges.clear();
      delegate.go(_home);
      await Future<void>.delayed(Duration.zero);
      expect(delegate.stack, [_other]);
      expect(router.stackChanges, isEmpty);

      check.complete(_home);
      await Future<void>.delayed(Duration.zero);

      expect(delegate.stack, [_home]);
      expect(router.stackChanges, ['[/other] => [/home]']);
      expect(calls, isEmpty);
      expect(await detailResult, isNull);
      expect(await confirmationResult, isNull);
    },
  );

  test('a cached route follows the router redirect before its guard', () async {
    final router = AuthRouter();
    final calls = <String>[];
    const privateRoute = TestRoute('/private');
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(
        router: router,
        routeBranch: (route) =>
            identical(route, privateRoute) ? _branch(_home) : _branch(route),
        resolveGuards: (route) => [RecordingGuard(route.location, calls)],
      ),
    );
    addTearDown(delegate.dispose);
    await delegate.debugWaitForScheduledRefresh();

    await delegate.setNewRoutePath(_home);
    final privateResult = delegate.push<void>(privateRoute);
    await Future<void>.delayed(Duration.zero);
    delegate.go(_other);
    await Future<void>.delayed(Duration.zero);

    calls.clear();
    router.isAuthenticated = false;
    delegate.go(_home);
    await Future<void>.delayed(Duration.zero);

    expect(delegate.stack.map((route) => route.location), ['/login']);
    expect(calls, ['/home', '/login']);
    delegate.dispose();
    await expectLater(privateResult, completion(isNull));
  });

  test('newer navigation supersedes a pending branch guard', () async {
    final check = Completer<TestRoute?>();
    var rechecking = false;
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(
        routeBranch: _branch,
        resolveGuards: (route) => route == _detail && rechecking
            ? [BlockingGuard(check.future)]
            : const [],
      ),
    );
    addTearDown(delegate.dispose);
    await delegate.debugWaitForScheduledRefresh();

    await delegate.setNewRoutePath(_home);
    final detailResult = delegate.push<void>(_detail);
    delegate.go(_other);

    rechecking = true;
    delegate.go(_home);
    await Future<void>.delayed(Duration.zero);
    expect(delegate.stack, [_other]);

    const next = TestRoute('/next');
    delegate.go(next);
    check.complete(null);
    await Future<void>.delayed(Duration.zero);

    expect(delegate.stack, [next]);
    delegate.dispose();
    await expectLater(detailResult, completion(isNull));
  });

  test(
    'cross-branch push guards the cached stack and follows its redirect',
    () async {
      final calls = <String>[];
      const fallback = TestRoute('/other/fallback');
      var rechecking = false;
      final delegate = GeneratedRouterDelegate<TestRoute>(
        runtimeConfig(
          routeBranch: _branch,
          resolveGuards: (route) => route == _home && rechecking
              ? [RedirectRecordingGuard(route.location, calls, fallback)]
              : const [],
        ),
      );
      addTearDown(delegate.dispose);
      await delegate.debugWaitForScheduledRefresh();

      await delegate.setNewRoutePath(_home);
      delegate.go(_other);
      final existing = delegate.push<String>(_otherDetail);
      final keys = delegate.debugPageKeys;

      rechecking = true;
      final requested = delegate.push<int>(_detail);
      await Future<void>.delayed(Duration.zero);

      expect(calls, [_home.location]);
      expect(delegate.stack, [_other, _otherDetail, fallback]);
      expect(delegate.debugPageKeys.take(2), keys);
      expect(delegate.pop(42), isTrue);
      expect(await requested, 42);
      expect(delegate.pop('saved'), isTrue);
      expect(await existing, 'saved');
    },
  );

  test('cross-branch replace checks the entries it retains', () async {
    final calls = <String>[];
    final delegate = GeneratedRouterDelegate<TestRoute>(
      runtimeConfig(
        routeBranch: _branch,
        resolveGuards: (route) => [RecordingGuard(route.location, calls)],
      ),
    );
    addTearDown(delegate.dispose);
    await delegate.debugWaitForScheduledRefresh();

    await delegate.setNewRoutePath(_home);
    final rootKey = delegate.debugPageKeys.first;
    final detail = delegate.push<void>(_detail);
    await Future<void>.delayed(Duration.zero);
    delegate.go(_other);
    await Future<void>.delayed(Duration.zero);

    calls.clear();
    delegate.replace(_confirmation);
    await Future<void>.delayed(Duration.zero);

    expect(calls, [_confirmation.location, _home.location]);
    expect(delegate.stack, [_home, _confirmation]);
    expect(delegate.debugPageKeys.first, same(rootKey));
    await expectLater(detail, completion(isNull));
  });

  test(
    'cached-route redirect does not expose a third branch unchecked',
    () async {
      final calls = <String>[];
      const fallback = TestRoute('/other/fallback');
      var rechecking = false;
      final delegate = GeneratedRouterDelegate<TestRoute>(
        runtimeConfig(
          routeBranch: _branch,
          resolveGuards: (route) => route == _home && rechecking
              ? [RedirectRecordingGuard(route.location, calls, fallback)]
              : const [],
        ),
      );
      addTearDown(delegate.dispose);
      await delegate.debugWaitForScheduledRefresh();

      await delegate.setNewRoutePath(_home);
      final homeDetail = delegate.push<void>(_detail);
      delegate.go(_other);
      final detail = delegate.push<void>(_otherDetail);
      delegate.go(const TestRoute('/safe'));

      rechecking = true;
      delegate.replace(_confirmation);
      await Future<void>.delayed(Duration.zero);

      expect(calls, [_home.location]);
      expect(delegate.stack, [fallback]);
      await expectLater(detail, completion(isNull));
      delegate.dispose();
      await expectLater(homeDetail, completion(isNull));
    },
  );
}
