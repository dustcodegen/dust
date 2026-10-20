import 'package:dust_dart/derive.dart';
import 'package:dust_flutter/state.dart';

part 'counter.g.dart';

/// State declared entirely through primary parameters.
@Derive([Eq(), CopyWith()])
class const CounterState({
  /// Current count.
  final int count = 0,
}) with _$CounterState {
  /// Creates the initial or updated state.
  this;
}

/// Typed arguments for the generated ViewModel.
final class const CounterArgs() extends ViewModelArgs {
  /// Creates counter arguments.
  this;
}

/// Counter forwarding its primary super parameter to the generated base.
@ViewModel(state: CounterState, args: CounterArgs)
class Counter(super.args) extends $Counter {
  /// Creates a counter.
  this;

  /// Updates through the existing generated copy API.
  void increment() => emit(state.copyWith(count: state.count + 1));
}
