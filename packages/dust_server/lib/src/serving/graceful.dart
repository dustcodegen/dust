import 'dart:async';
import 'dart:io';

import 'package:shelf/shelf.dart';
import 'package:shelf/shelf_io.dart' as shelf_io;

import '../router/middleware.dart';
import '../router/router_base.dart';
import '../stream_lifecycle.dart';
import 'background.dart';

/// A running server that can be stopped without dropping work.
///
/// `HttpServer.close` stops accepting and leaves active connections open
/// during unforced shutdown. The handle tracks requests still in flight.
final class ServerHandle {
  ServerHandle._(this._server, this._inFlight, this._background, this._layers)
      : address = _server.address,
        port = _server.port;

  final HttpServer _server;
  final _InFlight _inFlight;
  final BackgroundTasks? _background;
  final List<DisposableLayer> _layers;
  Future<bool>? _closing;

  /// The address the server bound to.
  ///
  /// Captured when the server started, so it stays readable after [close];
  /// asking `HttpServer` directly throws once the socket is gone.
  final InternetAddress address;

  /// The port the server bound to.
  final int port;

  /// How many requests are being handled right now.
  int get inFlight => _inFlight.count;

  /// How many background tasks are still running, when a registry was passed.
  int get pendingTasks => _background?.pending ?? 0;

  /// Stops accepting, then drains requests, tasks, and layer disposal.
  ///
  /// The first call sets the [drain] budget after the listener stops.
  /// Later calls share its result.
  /// Returns `true` when cleanup completes within that budget, or `false`
  /// while cleanup continues. Layer disposal starts when requests and
  /// background tasks finish or the deadline passes, whichever comes first.
  Future<bool> close({Duration drain = const Duration(seconds: 30)}) =>
      _closing ??= _close(drain);

  Future<bool> _close(Duration drain) async {
    // Return only after the OS has released the listening socket.
    await _server.close();

    Future<void>? disposal;
    Future<void> disposeOnce() => disposal ??= _disposeLayers();

    Future<void> drainAndDispose() async {
      await _inFlight.idle;
      await drainBackgroundTasks(_background);
      await disposeOnce();
    }

    return drainAndDispose().then((_) => true).timeout(
      drain,
      onTimeout: () {
        unawaited(drainBackgroundTasks(_background));
        // A pending request can depend on a layer closing its resource.
        unawaited(disposeOnce());
        return false;
      },
    );
  }

  /// Releases every [DisposableLayer] on the router, once.
  ///
  /// A layer that throws does not stop the others: shutdown has already
  /// stopped accepting, and losing the rest of the teardown to one bad
  /// `dispose` is worse than the error itself.
  Future<void> _disposeLayers() async {
    for (final layer in _layers) {
      try {
        await layer.dispose();
      } catch (_) {
        // Reported by the layer if it wants to be; shutdown continues.
      }
    }
    _layers.clear();
  }
}

/// Serves [router], counting requests so shutdown can wait for them.
///
/// [shared] lets several isolates bind the same port, which is what
/// `serveIsolates` uses; on its own a single server has no reason to set it.
///
/// [background] is drained alongside the requests. When it is omitted, a
/// [BackgroundTasks] attached to [router] with `withState` is used instead — so
/// a clustered server, whose registry is built inside each isolate, needs no
/// extra wiring to be drained.
///
/// ```dart
/// final server = await serve(app, InternetAddress.anyIPv4, 8080);
/// await ProcessSignal.sigterm.watch().first;
/// await server.close(drain: const Duration(seconds: 15));
/// ```
Future<ServerHandle> serve(
  Router router,
  InternetAddress address,
  int port, {
  SecurityContext? securityContext,
  bool shared = false,
  BackgroundTasks? background,
}) async {
  final inFlight = _InFlight();
  // Falling back to the router's own state means `withState(tasks)` is enough,
  // and a clustered server — where the registry is built inside each isolate and
  // cannot be handed in from outside — drains its tasks without any extra
  // wiring.
  final tasks = background ?? backgroundTasksIn(router);
  final handler = router.handler;

  final server = await shelf_io.serve(
    (request) async {
      inFlight.enter();
      try {
        final response = await handler(request);
        return _flushIfStreamed(
          response.change(
            body: finishWithStream(response.read(), inFlight.leave),
          ),
        );
      } on Object {
        inFlight.leave();
        rethrow;
      }
    },
    address,
    port,
    securityContext: securityContext,
    shared: shared,
  );

  return ServerHandle._(server, inFlight, tasks, disposableLayersIn(router));
}

/// Turns off output buffering for a response whose length is not known.
///
/// `shelf` buffers a streamed body and flushes when the stream ends, which for a
/// download makes the client wait for all of it and for an endless stream means
/// never. `streamed` and `eventStream` opt out themselves; this covers a
/// `Response` built by hand, including one from a mounted third-party handler
/// that has never heard of Dust.
///
/// Only when the length is unknown. A response with a `content-length` is a
/// single write, where buffering costs nothing and helps.
///
/// A response that set the key itself is left alone, so `bufferOutput: true` on
/// a chunked response remains sayable — worth having for a body that arrives as
/// very many tiny chunks, where one write each is slower than one write.
Response _flushIfStreamed(Response response) {
  if (response.contentLength != null) return response;
  if (response.context.containsKey(_bufferOutputKey)) return response;

  return response.change(context: {_bufferOutputKey: false});
}

/// The context key `shelf_io` reads to decide whether to buffer.
const _bufferOutputKey = 'shelf.io.buffer_output';

final class _InFlight {
  int count = 0;
  Completer<void>? _idle;

  void enter() => count++;

  void leave() {
    count--;
    if (count == 0) {
      _idle?.complete();
      _idle = null;
    }
  }

  Future<void> get idle =>
      count == 0 ? Future<void>.value() : (_idle ??= Completer<void>()).future;
}
