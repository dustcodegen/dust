import 'dart:async';

import 'package:dust_dart/fp.dart';
import 'package:shelf/shelf.dart';

import '../extraction/extractable.dart';
import '../response/error_reporting.dart';
import '../response/rejection.dart';
import '../router/middleware.dart';

/// The request context key carrying cooperative timeout cancellation.
const requestCancellationContextKey = 'dust_server/request_cancellation';

/// A cooperative signal fired when [RequestTimeout] reaches its deadline.
///
/// Dart cannot forcibly stop an arbitrary future. Handlers and dependencies
/// that support cancellation can observe [whenCancelled] and stop their own
/// work; other work continues after the 503 response.
final class RequestCancellation {
  RequestCancellation._();

  final Completer<void> _cancelled = Completer<void>();

  /// Whether the request deadline has passed.
  bool get isCancelled => _cancelled.isCompleted;

  /// Completes once the request deadline passes.
  Future<void> get whenCancelled => _cancelled.future;

  void _cancel() {
    if (!_cancelled.isCompleted) _cancelled.complete();
  }
}

/// Reads the cancellation signal installed by [RequestTimeout], if any.
RequestCancellation? requestCancellationOf(Request request) {
  final value = request.context[requestCancellationContextKey];
  return value is RequestCancellation ? value : null;
}

/// Extracts the signal for a generated or handwritten handler.
///
/// Generated handlers use
/// `@Extract(RequestCancellationExtractable) RequestCancellation cancellation`.
final class RequestCancellationExtractable
    implements FromRequestParts<RequestCancellation> {
  /// Creates the extractor used by generated handlers.
  const RequestCancellationExtractable();

  @override
  Future<Result<RequestCancellation, Rejection>> extract(
    Request request,
  ) async {
    final cancellation = requestCancellationOf(request);
    if (cancellation == null) {
      return const Err(
        Rejection.internal('RequestTimeout did not install cancellation'),
      );
    }
    return Ok(cancellation);
  }
}

/// Gives every request below it a deadline.
///
/// Without one, a handler that never completes holds its connection forever.
/// The body limit bounds how much a request can send; this bounds how long it
/// can take.
///
/// > **It bounds producing the response, not sending it.** The budget covers the
/// > handler returning a [Response]. A handler that returns immediately with a
/// > streamed body — an event stream, a large download — has already satisfied
/// > it, and the stream then runs for as long as it likes. Adding this layer
/// > above an SSE endpoint protects nothing there, and the connection is held
/// > until the stream ends or the client leaves. Bound those in the stream
/// > itself, with `take`, a timeout on the source, or a keep-alive the client
/// > answers.
///
/// ```dart
/// final app = Router()
///   ..layer(const RequestTimeout(Duration(seconds: 30)))
///   ..merge(routes);
/// ```
final class RequestTimeout implements Layer {
  /// Fails a request that takes longer than [budget].
  const RequestTimeout(this.budget, {this.onTimeout});

  /// How long a handler may take.
  final Duration budget;

  /// Called when the budget runs out, before the 503 goes back.
  ///
  /// The handler keeps running: Dart cannot cancel it. This is the hook for
  /// recording that it happened.
  final void Function(Request request)? onTimeout;

  @override
  Middleware toMiddleware() {
    if (budget <= Duration.zero) {
      throw ArgumentError.value(budget, 'budget', 'must be positive');
    }

    return (Handler inner) {
      return (Request request) {
        final cancellation = RequestCancellation._();
        final timed = request.change(
          context: {requestCancellationContextKey: cancellation},
        );
        return Future.sync(() => inner(timed)).timeout(
          budget,
          onTimeout: () {
            cancellation._cancel();
            try {
              onTimeout?.call(request);
            } on Object catch (error, stack) {
              ServerErrors.report(error, stack);
            }
            return Rejection.status(
              503,
              'the request did not complete within '
              '${budget.inMilliseconds}ms',
            ).intoResponse();
          },
        );
      };
    };
  }
}
