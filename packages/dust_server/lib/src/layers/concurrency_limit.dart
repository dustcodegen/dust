import 'dart:async';
import 'dart:collection';

import 'package:shelf/shelf.dart';

import '../response/rejection.dart';
import '../router/middleware.dart';
import 'timeout.dart';

/// Bounds work executing inside the application and sheds excess load.
///
/// Requests above [maxConcurrentRequests] wait only while the bounded queue has
/// room. A full queue answers 503 before routing, extraction, or the handler
/// runs. The HTTP server has already accepted the request; put a connection
/// limit at the reverse proxy when admission must happen before HTTP parsing.
final class ConcurrencyLimit implements Layer {
  /// Creates a bounded application-level concurrency limit.
  const ConcurrencyLimit({
    required this.maxConcurrentRequests,
    this.maxQueuedRequests = 0,
    this.retryAfter = const Duration(seconds: 1),
  });

  /// The most requests allowed to execute below this layer at once.
  final int maxConcurrentRequests;

  /// The most requests allowed to wait for execution.
  final int maxQueuedRequests;

  /// The delay advertised on an overload response.
  final Duration retryAfter;

  @override
  Middleware toMiddleware() {
    if (maxConcurrentRequests < 1) {
      throw ArgumentError.value(
        maxConcurrentRequests,
        'maxConcurrentRequests',
        'must be at least one',
      );
    }
    if (maxQueuedRequests < 0) {
      throw ArgumentError.value(
        maxQueuedRequests,
        'maxQueuedRequests',
        'must not be negative',
      );
    }
    if (retryAfter < const Duration(seconds: 1)) {
      throw ArgumentError.value(
        retryAfter,
        'retryAfter',
        'must be at least one second',
      );
    }

    final gate = _ConcurrencyGate(maxConcurrentRequests);
    return (Handler inner) {
      return (Request request) async {
        final admission = gate.admit(
          maxQueuedRequests,
          requestCancellationOf(request),
        );
        if (admission == null) return _overloaded(retryAfter);
        if (!await admission) return _overloaded(retryAfter);

        try {
          return await inner(request);
        } finally {
          gate.leave();
        }
      };
    };
  }
}

Response _overloaded(Duration retryAfter) {
  return const Rejection.status(503, 'server is at capacity')
      .intoResponse()
      .change(headers: {'retry-after': '${retryAfter.inSeconds}'});
}

final class _ConcurrencyGate {
  _ConcurrencyGate(this.maximum);

  final int maximum;
  final Queue<_Waiter> waiting = Queue<_Waiter>();
  int active = 0;

  Future<bool>? admit(int queueLimit, RequestCancellation? cancellation) {
    if (cancellation?.isCancelled ?? false) return Future.value(false);
    if (active < maximum) {
      active++;
      return Future.value(true);
    }
    if (waiting.length >= queueLimit) return null;

    final waiter = _Waiter();
    waiting.addLast(waiter);
    if (cancellation case final signal?) {
      unawaited(signal.whenCancelled.then((_) => cancel(waiter)));
    }
    return waiter.admitted.future;
  }

  void cancel(_Waiter waiter) {
    if (waiting.remove(waiter) && !waiter.admitted.isCompleted) {
      waiter.admitted.complete(false);
    }
  }

  void leave() {
    while (waiting.isNotEmpty) {
      final next = waiting.removeFirst();
      if (!next.admitted.isCompleted) {
        next.admitted.complete(true);
        return;
      }
    }
    active--;
  }
}

final class _Waiter {
  final admitted = Completer<bool>();
}
