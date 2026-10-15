import 'dart:io';
import 'dart:ui';

/// Summarizes observations, not an application performance guarantee.
Map<String, Object?> summarizeMicros(Iterable<int> values) {
  final sorted = values.toList()..sort();
  return {
    'count': sorted.length,
    'p50Us': sorted.isEmpty ? null : sorted[(sorted.length * .5).ceil() - 1],
    'p95Us': sorted.isEmpty ? null : sorted[(sorted.length * .95).ceil() - 1],
    'maxUs': sorted.isEmpty ? null : sorted.last,
  };
}

/// Collects actual engine-delivered frame timings and process RSS.
final class ProbeMetrics {
  /// Captures the initial resident memory observation.
  ProbeMetrics() : rssBefore = ProcessInfo.currentRss;

  /// Initial process resident bytes, not Dart heap size.
  final int rssBefore;

  /// Engine timings delivered during this probe.
  final frames = <FrameTiming>[];

  /// UI animation builds counted on the caller isolate.
  int uiFrames = 0;

  /// Register with the scheduler; remove the same callback on teardown.
  void record(List<FrameTiming> values) => frames.addAll(values);

  /// Includes raw samples so summary values can be independently checked.
  Map<String, Object?> report() => {
        'rssBeforeBytes': rssBefore,
        'rssAfterBytes': ProcessInfo.currentRss,
        'maxRssBytes': ProcessInfo.maxRss,
        'uiFrames': uiFrames,
        'build':
            summarizeMicros(frames.map((f) => f.buildDuration.inMicroseconds)),
        'raster':
            summarizeMicros(frames.map((f) => f.rasterDuration.inMicroseconds)),
        'buildSamplesUs': [
          for (final f in frames) f.buildDuration.inMicroseconds
        ],
        'rasterSamplesUs': [
          for (final f in frames) f.rasterDuration.inMicroseconds
        ],
      };
}
