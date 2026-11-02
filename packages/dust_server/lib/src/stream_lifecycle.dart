import 'dart:async';

/// Runs [finished] when [source] is consumed or its listener cancels.
Stream<T> finishWithStream<T>(
  Stream<T> source,
  void Function() finished,
) {
  final controller = StreamController<T>(sync: true);
  StreamSubscription<T>? subscription;
  var ended = false;

  void finish() {
    if (ended) return;
    ended = true;
    finished();
  }

  controller.onListen = () {
    subscription = source.listen(
      controller.add,
      onError: controller.addError,
      onDone: () {
        finish();
        unawaited(controller.close());
      },
    );
  };
  controller.onPause = () => subscription?.pause();
  controller.onResume = () => subscription?.resume();
  controller.onCancel = () {
    finish();
    return subscription?.cancel();
  };

  return controller.stream;
}
