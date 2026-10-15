import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../integration_test/support/probe_config.dart';
import '../integration_test/support/probe_metrics.dart';
import '../integration_test/support/probe_ui.dart';

void main() {
  test('probe accepts only explicit safe run identifiers and phases', () {
    validateProbeConfig('local-123_test', 'write');
    validateProbeConfig('local-123_test', 'verify');
    for (final invalid in ['', '.', '..', '../app', '/tmp/app', 'a/b', 'a b']) {
      expect(() => validateProbeConfig(invalid, 'write'), throwsArgumentError);
    }
    expect(() => validateProbeConfig('a' * 65, 'write'), throwsArgumentError);
    expect(() => validateProbeConfig('local', 'delete'), throwsArgumentError);
  });
  test('frame statistics retain missing data and nearest-rank percentiles', () {
    expect(summarizeMicros([]), {
      'count': 0,
      'p50Us': null,
      'p95Us': null,
      'maxUs': null,
    });
    expect(summarizeMicros([7]), {
      'count': 1,
      'p50Us': 7,
      'p95Us': 7,
      'maxUs': 7,
    });
    expect(summarizeMicros([for (var i = 100; i > 0; i--) i]), {
      'count': 100,
      'p50Us': 50,
      'p95Us': 95,
      'maxUs': 100,
    });
  });
  test('invalid storage configuration fails before accessing the platform',
      () async {
    await expectLater(probeDirectory('../app', 'write'), throwsArgumentError);
    await expectLater(probeDirectory('local', 'delete'), throwsArgumentError);
  });
  testWidgets('probe animation builds frames and disposes its ticker',
      (tester) async {
    var frames = 0;
    await tester.pumpWidget(ProbeUi(onFrame: () => frames++));
    final first = frames;
    await tester.pump(const Duration(milliseconds: 100));
    expect(frames, greaterThan(first));
    await tester.pumpWidget(const SizedBox.shrink());
  });
}
