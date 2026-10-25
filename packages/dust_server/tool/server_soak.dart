import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:dust_server/server.dart';

/// Sustained local traffic and RSS probe.
///
/// Usage: `dart run tool/server_soak.dart [seconds] [concurrency] [max-rss-mb]`.
Future<void> main(List<String> arguments) async {
  final seconds = _positive(arguments, 0, 3600);
  final concurrency = _positive(arguments, 1, 32);
  final maxGrowthMb = _positive(arguments, 2, 64);
  if (seconds == null || concurrency == null || maxGrowthMb == null) {
    stderr.writeln(
      'usage: dart run tool/server_soak.dart '
      '[positive-seconds] [positive-concurrency] [positive-max-rss-growth-mb]',
    );
    exitCode = 64;
    return;
  }

  var handlerErrors = 0;
  final app = productionRouter(
    bodyLimit: 1024,
    maxConcurrentRequests: concurrency,
    maxQueuedRequests: concurrency,
    requestTimeout: const Duration(seconds: 5),
    onRecord: (_) {},
    onError: (_, __) => handlerErrors++,
  )..route('/work', post((request) async {
      await request.read().drain<void>();
      return noContent();
    }));
  final server = await serve(app, InternetAddress.loopbackIPv4, 0);
  final uri = Uri.parse('http://${server.address.host}:${server.port}/work');
  final client = HttpClient()..maxConnectionsPerHost = concurrency;

  await _runLoad(client, uri, const Duration(seconds: 5), concurrency);
  final baselineRss = ProcessInfo.currentRss;
  var peakRss = baselineRss;
  final sampler = Timer.periodic(const Duration(seconds: 1), (_) {
    final rss = ProcessInfo.currentRss;
    if (rss > peakRss) peakRss = rss;
  });
  final result = await _runLoad(
    client,
    uri,
    Duration(seconds: seconds),
    concurrency,
  );
  sampler.cancel();
  client.close(force: true);
  final drained = await server.close(drain: const Duration(seconds: 10));
  final finalRss = ProcessInfo.currentRss;
  if (finalRss > peakRss) peakRss = finalRss;
  final growth = peakRss - baselineRss;

  stdout.writeln(
    jsonEncode({
      'seconds': seconds,
      'concurrency': concurrency,
      'requests': result.requests,
      'failures': result.failures,
      'handlerErrors': handlerErrors,
      'drained': drained,
      'baselineRssBytes': baselineRss,
      'peakRssBytes': peakRss,
      'rssGrowthBytes': growth,
      'maxRssGrowthBytes': maxGrowthMb * 1024 * 1024,
    }),
  );
  if (result.failures != 0 || handlerErrors != 0 || !drained) exitCode = 1;
  if (growth > maxGrowthMb * 1024 * 1024) exitCode = 1;
}

int? _positive(List<String> arguments, int index, int fallback) {
  if (index >= arguments.length) return fallback;
  final value = int.tryParse(arguments[index]);
  return value != null && value > 0 ? value : null;
}

Future<_LoadResult> _runLoad(
  HttpClient client,
  Uri uri,
  Duration duration,
  int concurrency,
) async {
  var requests = 0;
  var failures = 0;
  final elapsed = Stopwatch()..start();

  await Future.wait(List.generate(concurrency, (_) async {
    while (elapsed.elapsed < duration) {
      try {
        final request = await client.postUrl(uri);
        request.headers.contentType = ContentType.json;
        request.write('{"value":1}');
        final response = await request.close();
        await response.drain<void>();
        requests++;
        if (response.statusCode != 204) failures++;
      } on Object {
        failures++;
      }
    }
  }));
  return _LoadResult(requests, failures);
}

final class _LoadResult {
  const _LoadResult(this.requests, this.failures);

  final int requests;
  final int failures;
}
