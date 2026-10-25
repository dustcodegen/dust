import 'package:shelf/shelf.dart';

import '../router/router_base.dart';
import 'access_log.dart';
import 'concurrency_limit.dart';
import 'normalize_path.dart';
import 'request_id.dart';
import 'timeout.dart';

/// Builds an outer router with Dust's minimum production safety policy.
///
/// Merge generated modules or handwritten routers into the result. Limits stay
/// explicit because safe values depend on the deployment and handler costs.
Router productionRouter({
  required int bodyLimit,
  required int maxConcurrentRequests,
  required Duration requestTimeout,
  required void Function(AccessRecord record) onRecord,
  required void Function(Object error, StackTrace stack) onError,
  int maxQueuedRequests = 0,
  Duration retryAfter = const Duration(seconds: 1),
  void Function(Request request)? onTimeout,
}) {
  if (bodyLimit < 0) {
    throw ArgumentError.value(bodyLimit, 'bodyLimit', 'must not be negative');
  }

  return Router(bodyLimit: bodyLimit, onError: onError)
    ..layer(const RequestId())
    ..layer(AccessLog(onRecord))
    ..layer(const SecurityHeaders())
    // The timeout owns cancellation. Keeping concurrency inside it means work
    // that ignores cancellation still occupies its slot until it really ends.
    ..layer(RequestTimeout(requestTimeout, onTimeout: onTimeout))
    ..layer(
      ConcurrencyLimit(
        maxConcurrentRequests: maxConcurrentRequests,
        maxQueuedRequests: maxQueuedRequests,
        retryAfter: retryAfter,
      ),
    );
}
