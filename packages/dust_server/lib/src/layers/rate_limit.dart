import 'dart:math' as math;

import 'package:shelf/shelf.dart';

import '../response/rejection.dart';
import '../router/middleware.dart';

/// Limits requests per application-defined client identity.
///
/// [keyBy] is required because Dust cannot safely guess whether an application
/// trusts a proxy header, an API key, or an authenticated subject.
final class RateLimit implements Layer {
  /// Creates a token-bucket rate limit with bounded identity storage.
  const RateLimit({
    required this.maxRequests,
    required this.window,
    required this.keyBy,
    this.maxKeys = 10000,
  });

  /// The full-bucket capacity and number of tokens restored per [window].
  final int maxRequests;

  /// How long it takes to restore [maxRequests] tokens.
  final Duration window;

  /// Returns the trusted identity whose bucket should be charged.
  final String Function(Request request) keyBy;

  /// Maximum identities retained at once.
  final int maxKeys;

  @override
  Middleware toMiddleware() {
    if (maxRequests < 1) {
      throw ArgumentError.value(maxRequests, 'maxRequests', 'must be positive');
    }
    if (window <= Duration.zero) {
      throw ArgumentError.value(window, 'window', 'must be positive');
    }
    if (maxKeys < 1) {
      throw ArgumentError.value(maxKeys, 'maxKeys', 'must be positive');
    }

    final clock = Stopwatch()..start();
    final buckets = <String, _Bucket>{};

    return (Handler inner) {
      return (Request request) {
        final now = clock.elapsedMicroseconds;
        final key = keyBy(request);
        var bucket = buckets[key];

        if (bucket == null) {
          buckets.removeWhere(
            (_, candidate) =>
                candidate.available(now, maxRequests, window) >= maxRequests,
          );
          if (buckets.length >= maxKeys) return _limited(window);
          bucket = _Bucket(maxRequests.toDouble(), now);
          buckets[key] = bucket;
        }

        final available = bucket.available(now, maxRequests, window);
        if (available < 1) {
          return _limited(bucket.retryAfter(now, maxRequests, window));
        }
        bucket
          ..tokens = available - 1
          ..updatedAt = now;
        return inner(request);
      };
    };
  }
}

Response _limited(Duration retryAfter) {
  final seconds = math.max(1, (retryAfter.inMilliseconds / 1000).ceil());
  return const Rejection.status(429, 'too many requests')
      .intoResponse()
      .change(headers: {'retry-after': '$seconds'});
}

final class _Bucket {
  _Bucket(this.tokens, this.updatedAt);

  double tokens;
  int updatedAt;

  double available(int now, int maximum, Duration window) {
    final elapsed = now - updatedAt;
    final restored = elapsed * maximum / window.inMicroseconds;
    return math.min(maximum.toDouble(), tokens + restored);
  }

  Duration retryAfter(int now, int maximum, Duration window) {
    final missing = 1 - available(now, maximum, window);
    return Duration(
      microseconds: (missing * window.inMicroseconds / maximum).ceil(),
    );
  }
}
