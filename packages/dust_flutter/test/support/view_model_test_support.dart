import 'dart:async';

import 'package:dust_flutter/state.dart';

final class TestArgs extends ViewModelArgs {
  const TestArgs({super.observer});
}

final class CounterViewModel extends ViewModelBase<int, TestArgs> {
  CounterViewModel({StateObserver? observer})
    : super(TestArgs(observer: observer), initialState: 0);

  static const Object testAction = Object();

  void setCount(int next) {
    emit(next);
  }

  void emitTestEffect(Object effect) {
    emitEffect(effect);
  }

  ViewModelActionToken beginTestAction() {
    return beginAction(testAction);
  }

  bool isCurrentTestAction(ViewModelActionToken token) {
    return isCurrentAction(token);
  }

  Future<bool> runTestAction(
    Future<int> Function() run, {
    void Function(Object error, StackTrace stackTrace)? onError,
  }) {
    return runAction<int>(
      testAction,
      onStart: () => emit(-1),
      run: run,
      onSuccess: emit,
      onError: onError,
    );
  }
}

final class InitViewModel extends ViewModelBase<int, TestArgs> {
  InitViewModel() : super(const TestArgs(), initialState: 0);

  final initCompleters = <Completer<void>>[];
  var initCalls = 0;

  @override
  Future<void> onInit() {
    initCalls += 1;
    final completer = Completer<void>();
    initCompleters.add(completer);
    return completer.future;
  }
}

final class RecordingStateObserver implements StateObserver {
  final effects = <Object>[];

  @override
  void onChanged(Object viewModel, Object? previous, Object? next) {}

  @override
  void onEffect(Object viewModel, Object effect) {
    effects.add(effect);
  }
}
