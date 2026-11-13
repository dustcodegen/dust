import 'dart:async';

import 'package:dust_flutter/state.dart';
import 'package:flutter_test/flutter_test.dart';

final class _ViewModel extends ViewModelBase<int, ViewModelArgs> {
  _ViewModel(this._initialize) : super(const ViewModelArgs(), initialState: 0);

  final FutureOr<void> Function(_ViewModel) _initialize;

  @override
  FutureOr<void> onInit() => _initialize(this);
}

void main() {
  test('retryInit recovers from a synchronous onInit failure', () async {
    final failure = StateError('initialization failed');
    var attempts = 0;
    final viewModel = _ViewModel((_) {
      attempts++;
      if (attempts == 1) throw failure;
    });
    addTearDown(viewModel.dispose);

    await expectLater(viewModel.init(), throwsA(same(failure)));
    await viewModel.init();
    expect(attempts, 1);

    await viewModel.retryInit();
    await viewModel.retryInit();
    expect(attempts, 2);
  });

  test('init and retryInit share an attempt during onInit', () async {
    final setup = Completer<void>();
    var attempts = 0;
    late Future<void> nestedInit;
    late Future<void> nestedRetry;
    final viewModel = _ViewModel((model) {
      attempts++;
      if (attempts == 1) {
        nestedInit = model.init();
        nestedRetry = model.retryInit();
      }
      return setup.future;
    });
    addTearDown(viewModel.dispose);

    final pending = viewModel.init();
    setup.complete();

    expect(attempts, 1);
    expect(nestedInit, same(pending));
    expect(nestedRetry, same(pending));
    await pending;
  });
}
