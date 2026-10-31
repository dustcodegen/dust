import 'dart:async';
import 'dart:ffi';
import 'dart:io';

typedef _CallbackNative = Void Function(Uint64);

const _total = 500000;
const _windows = [1, 8, 32, 128, 512];

Future<void> main(List<String> arguments) async {
  if (arguments.length != 1) {
    stderr.writeln('usage: bridge_benchmark.dart path/to/libbridge.so');
    exitCode = 64;
    return;
  }
  final library = DynamicLibrary.open(arguments.single);
  final create = library.lookupFunction<
      Pointer<Void> Function(
        Pointer<NativeFunction<_CallbackNative>>,
        Uint64,
        Uint64,
      ),
      Pointer<Void> Function(
        Pointer<NativeFunction<_CallbackNative>>,
        int,
        int,
      )>(
    'dust_bridge_new',
  );
  final start = library.lookupFunction<Bool Function(Pointer<Void>),
      bool Function(Pointer<Void>)>(
    'dust_bridge_start',
  );
  final ack = library.lookupFunction<Void Function(Pointer<Void>, Uint64),
      void Function(Pointer<Void>, int)>('dust_bridge_ack');
  final elapsed = library.lookupFunction<Uint64 Function(Pointer<Void>),
      int Function(Pointer<Void>)>(
    'dust_bridge_elapsed_ns',
  );
  final free = library.lookupFunction<Void Function(Pointer<Void>),
      void Function(Pointer<Void>)>(
    'dust_bridge_free',
  );

  for (final window in _windows) {
    final done = Completer<void>();
    var received = 0;
    var checksum = 0;
    late Pointer<Void> bridge;
    late NativeCallable<_CallbackNative> callback;
    callback = NativeCallable<_CallbackNative>.listener((int id) {
      checksum ^= id;
      ack(bridge, id);
      received++;
      if (received == _total) done.complete();
    });

    bridge = create(callback.nativeFunction, _total, window);
    if (bridge == nullptr || !start(bridge)) {
      callback.close();
      stderr.writeln('failed to start bridge');
      exitCode = 1;
      return;
    }
    await done.future;
    final elapsedNs = elapsed(bridge);
    final rate = _total * 1000000000 / elapsedNs;
    stdout.writeln(
      'window=$window rate=${rate.toStringAsFixed(0)}/s '
      'elapsed=${(elapsedNs / 1000000).toStringAsFixed(1)}ms '
      'checksum=$checksum',
    );
    free(bridge);
    callback.close();
  }
}
