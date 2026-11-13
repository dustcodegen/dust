import 'dart:async';

import 'package:dust_flutter/state.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/view_model_test_support.dart';

void main() {
  test('invalidateSelf resets sync state to initial state', () {
    final viewModel = CounterViewModel()
      ..setCount(3)
      ..invalidateSelf();

    expect(viewModel.state, 0);
  });

  test(
    'invalidateSelf keeps old action tokens stale after action restarts',
    () {
      final viewModel = CounterViewModel();
      final oldToken = viewModel.beginTestAction();

      viewModel
        ..setCount(3)
        ..invalidateSelf();

      final newToken = viewModel.beginTestAction();

      expect(viewModel.isCurrentTestAction(oldToken), isFalse);
      expect(viewModel.isCurrentTestAction(newToken), isTrue);
    },
  );

  test('runAction ignores stale success result', () async {
    final viewModel = CounterViewModel();
    final firstCompleter = Completer<int>();
    final secondCompleter = Completer<int>();

    final first = viewModel.runTestAction(() => firstCompleter.future);
    expect(viewModel.state, -1);

    final second = viewModel.runTestAction(() => secondCompleter.future);
    expect(viewModel.state, -1);

    secondCompleter.complete(2);
    expect(await second, isTrue);
    expect(viewModel.state, 2);

    firstCompleter.complete(1);
    expect(await first, isFalse);
    expect(viewModel.state, 2);
  });

  test('runAction ignores stale error result', () async {
    final viewModel = CounterViewModel();
    final firstCompleter = Completer<int>();
    final secondCompleter = Completer<int>();

    final first = viewModel.runTestAction(
      () => firstCompleter.future,
      onError: (error, stackTrace) {
        fail('stale errors must not call onError: $error $stackTrace');
      },
    );
    final second = viewModel.runTestAction(() => secondCompleter.future);

    secondCompleter.complete(2);
    expect(await second, isTrue);

    firstCompleter.completeError(StateError('stale'));
    expect(await first, isFalse);
    expect(viewModel.state, 2);
  });

  test('runAction calls onError or propagates the current error', () async {
    final viewModel = CounterViewModel();
    final completer = Completer<int>();
    final error = StateError('failed');

    final action = viewModel.runTestAction(
      () => completer.future,
      onError: (error, _) {
        expect(error, isA<StateError>());
        viewModel.setCount(-2);
      },
    );

    completer.completeError(error);

    expect(await action, isTrue);
    expect(viewModel.state, -2);
    final unhandled = viewModel.runTestAction(() => Future<int>.error(error));
    await expectLater(unhandled, throwsA(same(error)));
  });

  test(
    'disposed view model drops pending and new actions and effects',
    () async {
      final observer = RecordingStateObserver();
      final viewModel = CounterViewModel(observer: observer);
      final pending = Completer<int>();
      final action = viewModel.runTestAction(() => pending.future);

      viewModel
        ..dispose()
        ..emitTestEffect('ignored')
        ..invalidateSelf();
      pending.complete(4);

      expect(await action, isFalse);
      expect(
        await viewModel.runTestAction(
          () async => fail('started after disposal'),
        ),
        isFalse,
      );
      expect(viewModel.state, -1);
      expect(observer.effects, isEmpty);
    },
  );

  test('init runs onInit once for concurrent calls', () async {
    final viewModel = InitViewModel();

    final first = viewModel.init();
    final second = viewModel.init();

    expect(viewModel.initCalls, 1);

    viewModel.initCompleters.single.complete();
    await Future.wait([first, second]);

    await viewModel.init();

    expect(viewModel.initCalls, 1);
  });

  test('init does not retry after onInit failure', () async {
    final viewModel = InitViewModel();

    final first = viewModel.init();
    viewModel.initCompleters.single.completeError(StateError('failed'));

    await expectLater(first, throwsA(isA<StateError>()));
    expect(viewModel.initCalls, 1);

    await viewModel.init();

    expect(viewModel.initCalls, 1);
  });

  test('retryInit retries after onInit failure', () async {
    final viewModel = InitViewModel();

    final first = viewModel.init();
    viewModel.initCompleters.single.completeError(StateError('failed'));

    await expectLater(first, throwsA(isA<StateError>()));
    expect(viewModel.initCalls, 1);

    final second = viewModel.retryInit();

    expect(viewModel.initCalls, 2);

    viewModel.initCompleters.last.complete();
    await second;

    await viewModel.retryInit();

    expect(viewModel.initCalls, 2);
  });

  test('emitEffect delivers raw effect to stream and observer', () async {
    final observer = RecordingStateObserver();
    final viewModel = CounterViewModel(observer: observer);
    final effects = <Object>[];
    final subscription = viewModel.effects.listen(effects.add);

    addTearDown(viewModel.dispose);
    addTearDown(subscription.cancel);

    final effect = Object();
    viewModel.emitTestEffect(effect);

    expect(observer.effects.single, same(effect));
    expect(effects, isEmpty);

    await pumpEventQueue();

    expect(effects.single, same(effect));
  });

  test('effects emitted before subscription are not replayed', () async {
    final viewModel = CounterViewModel();
    final effects = <Object>[];

    addTearDown(viewModel.dispose);

    viewModel.emitTestEffect('early');

    final subscription = viewModel.effects.listen(effects.add);
    addTearDown(subscription.cancel);

    await pumpEventQueue();

    expect(effects, isEmpty);
  });

  test(
    'effects are dropped when subscription cancels before delivery',
    () async {
      final viewModel = CounterViewModel();
      final effects = <Object>[];
      final subscription = viewModel.effects.listen(effects.add);

      addTearDown(viewModel.dispose);

      viewModel.emitTestEffect('toast');
      await subscription.cancel();
      await pumpEventQueue();

      expect(effects, isEmpty);
    },
  );

  test('emitEffect unwraps deprecated StateEffect wrapper', () async {
    final observer = RecordingStateObserver();
    final viewModel = CounterViewModel(observer: observer);
    final effects = <Object>[];
    final subscription = viewModel.effects.listen(effects.add);

    addTearDown(viewModel.dispose);
    addTearDown(subscription.cancel);

    final effect = Object();
    // ignore: deprecated_member_use_from_same_package
    viewModel.emitTestEffect(StateEffect(effect));
    await pumpEventQueue();

    expect(effects.single, same(effect));
    expect(observer.effects.single, same(effect));
  });
}
