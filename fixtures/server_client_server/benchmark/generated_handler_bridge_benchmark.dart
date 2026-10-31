import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:dust_server/server.dart';
import 'package:server_client_server/client_edges.dart';

typedef _CallbackNative = Void Function(Uint64);

const _total = 100000;
const _windows = [1, 8, 32, 128];
final List<String> _payloads = [
  jsonEncode({
    'display_name': 'Ship Dust',
    'order': 'newest',
    'metadata': {'source': 'benchmark'},
    'tags': ['dart', 'rust'],
  }),
  jsonEncode({
    'display_name': 'Profile Dust',
    'order': 'oldest',
    'metadata': {'source': 'benchmark'},
    'tags': ['server'],
  }),
];

Future<void> main(List<String> arguments) async {
  if (arguments.length != 1) {
    stderr.writeln('usage: generated_handler_bridge_benchmark.dart '
        'path/to/libbridge.so');
    exitCode = 64;
    return;
  }

  final bindings = _BridgeBindings(DynamicLibrary.open(arguments.single));
  final handler = Transport().routes().handler;
  for (final window in _windows) {
    await _measure(bindings, handler, window);
  }
}

Future<void> _measure(
  _BridgeBindings bindings,
  Handler handler,
  int window,
) async {
  final done = Completer<void>();
  var received = 0;
  var failures = 0;
  var checksum = 0;
  late Pointer<Void> bridge;
  late NativeCallable<_CallbackNative> callback;

  callback = NativeCallable<_CallbackNative>.listener((int id) {
    unawaited(() async {
      try {
        final response = await handler(_request(id));
        final body = await response.readAsString();
        if (response.statusCode != 201 || !body.contains('"id":"created"')) {
          failures++;
        }
        checksum += response.statusCode + body.length;
      } finally {
        bindings.ack(bridge, id);
        received++;
        if (received == _total) done.complete();
      }
    }());
  });

  bridge = bindings.create(callback.nativeFunction, _total, window);
  if (bridge == nullptr || !bindings.start(bridge)) {
    callback.close();
    throw StateError('failed to start bridge');
  }

  await done.future;
  final elapsedNs = bindings.elapsed(bridge);
  final rate = _total * 1000000000 / elapsedNs;
  stdout.writeln(
    'window=$window rate=${rate.toStringAsFixed(0)}/s '
    'elapsed=${(elapsedNs / 1000000).toStringAsFixed(1)}ms '
    'failures=$failures checksum=$checksum',
  );
  bindings.free(bridge);
  callback.close();

  if (failures != 0) {
    throw StateError('$failures generated-handler responses were invalid');
  }
}

Request _request(int id) => Request(
      'POST',
      Uri.parse('http://localhost/transport/typed'),
      headers: const {'content-type': 'application/json'},
      body: _payloads[id & 1],
    );

final class _BridgeBindings {
  _BridgeBindings(DynamicLibrary library)
      : create = library.lookupFunction<
            Pointer<Void> Function(
              Pointer<NativeFunction<_CallbackNative>>,
              Uint64,
              Uint64,
            ),
            Pointer<Void> Function(
              Pointer<NativeFunction<_CallbackNative>>,
              int,
              int,
            )>('dust_bridge_new'),
        start = library.lookupFunction<Bool Function(Pointer<Void>),
            bool Function(Pointer<Void>)>('dust_bridge_start'),
        ack = library.lookupFunction<Void Function(Pointer<Void>, Uint64),
            void Function(Pointer<Void>, int)>('dust_bridge_ack'),
        elapsed = library.lookupFunction<Uint64 Function(Pointer<Void>),
            int Function(Pointer<Void>)>('dust_bridge_elapsed_ns'),
        free = library.lookupFunction<Void Function(Pointer<Void>),
            void Function(Pointer<Void>)>('dust_bridge_free');

  final Pointer<Void> Function(
    Pointer<NativeFunction<_CallbackNative>>,
    int,
    int,
  ) create;
  final bool Function(Pointer<Void>) start;
  final void Function(Pointer<Void>, int) ack;
  final int Function(Pointer<Void>) elapsed;
  final void Function(Pointer<Void>) free;
}
