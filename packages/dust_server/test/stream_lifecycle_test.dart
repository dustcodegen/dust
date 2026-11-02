import 'dart:async';

import 'package:dust_server/src/stream_lifecycle.dart';
import 'package:test/test.dart';

void main() {
  test('finishes once when the source closes', () async {
    var finished = 0;
    final stream = finishWithStream(
      Stream.fromIterable([1, 2]),
      () => finished++,
    );

    expect(await stream.toList(), [1, 2]);
    expect(finished, 1);
  });

  test('forwards pause, resume, and cancellation', () async {
    var finished = 0;
    final source = StreamController<int>();
    final subscription = finishWithStream(
      source.stream,
      () => finished++,
    ).listen(null);

    subscription.pause();
    expect(source.isPaused, isTrue);
    subscription.resume();
    expect(source.isPaused, isFalse);
    await subscription.cancel();

    expect(source.hasListener, isFalse);
    expect(finished, 1);
  });
}
