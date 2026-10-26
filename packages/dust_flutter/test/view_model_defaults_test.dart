import 'package:dust_flutter/state.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

final class _Args extends ViewModelArgs {
  const _Args();
}

final class _DefaultViewModel extends ViewModelBase<int, _Args> {
  _DefaultViewModel() : super(const _Args(), initialState: 0);
}

void main() {
  test('default initialization completes once without work', () async {
    final viewModel = _DefaultViewModel();
    addTearDown(viewModel.dispose);

    await viewModel.init();
    await viewModel.init();

    expect(viewModel.state, 0);
  });

  test('logging observer reports changes and effects in debug mode', () {
    final messages = <String>[];
    final previousDebugPrint = debugPrint;
    debugPrint = (message, {wrapWidth}) {
      if (message != null) messages.add(message);
    };

    try {
      final observer = LoggingStateObserver();
      final viewModel = _DefaultViewModel();

      observer
        ..onChanged(viewModel, 0, 1)
        ..onEffect(viewModel, 'saved');

      expect(messages, [
        'STATE CHANGE: _DefaultViewModel',
        'FROM: 0',
        'TO:   1',
        'EFFECT: _DefaultViewModel -> saved',
      ]);
    } finally {
      debugPrint = previousDebugPrint;
    }
  });

  test('silent observer accepts changes and effects', () {
    final observer = SilentStateObserver();
    final viewModel = _DefaultViewModel();

    observer
      ..onChanged(viewModel, 0, 1)
      ..onEffect(viewModel, 'saved');
  });
}
