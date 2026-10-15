import 'dart:convert';
import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import 'support/probe_config.dart';
import 'support/probe_lifecycle.dart';
import 'support/probe_metrics.dart';
import 'support/probe_storage.dart';
import 'support/probe_ui.dart';

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized()
    ..framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;
  final lifecycle = ProbeLifecycle()..registerStatus();
  const run = String.fromEnvironment('DUST_SQLITE_PROBE_RUN');
  const phase = String.fromEnvironment('DUST_SQLITE_PROBE_PHASE');

  testWidgets('SQLite worker / OS lifecycle / $phase', (tester) async {
    final directory = await probeDirectory(run, phase);
    final path = '${directory.path}/probe.db';
    final evidence = File('${directory.path}/expected.json');
    Map<String, dynamic>? prior;
    if (phase == 'write') {
      expect(directory.existsSync(), isFalse,
          reason: 'Never reuse probe data.');
      await directory.create();
    } else {
      expect(FileSystemEntity.isLinkSync(directory.path), isFalse);
      expect(File(path).existsSync(), isTrue, reason: 'Relaunch must reopen.');
      expect(evidence.existsSync(), isTrue);
      prior = jsonDecode(await evidence.readAsString()) as Map<String, dynamic>;
      expect(prior['owner'], 'dust-sqlite-lifecycle-v1');
      expect(prior['run'], run);
      expect(prior['pid'], isNot(pid), reason: 'Require a new OS process.');
    }
    final metrics = ProbeMetrics();
    final scheduler = SchedulerBinding.instance
      ..addTimingsCallback(metrics.record);
    binding.addObserver(lifecycle);
    addTearDown(() {
      scheduler.removeTimingsCallback(metrics.record);
      binding.removeObserver(lifecycle);
    });
    await tester.pumpWidget(ProbeUi(onFrame: () => metrics.uiFrames++));
    expect(find.byKey(const ValueKey('sqlite-probe')), findsOneWidget);
    final report = <String, Object?>{
      'run': run,
      'phase': phase,
      'pid': pid,
      'platform': Platform.operatingSystem,
      'os': Platform.operatingSystemVersion,
      'dart': Platform.version,
      'flutter': const String.fromEnvironment('FLUTTER_VERSION'),
      'mode': kProfileMode ? 'profile' : (kReleaseMode ? 'release' : 'debug'),
      'probeDirectory': directory.path,
    };
    if (phase == 'write') {
      report['lockedOpening'] = await probeLockedOpening(
        '${directory.path}/locked.db',
        metrics,
      );
    }
    final db = await openProbe(path);
    late String history;
    try {
      report['engine'] = (await db.unsafe.fetch(
        'SELECT sqlite_version() AS version, sqlite_source_id() AS source',
        [],
      ))
          .unwrap()
          .map((row) => {
                'version': row.read<String>('version'),
                'sourceId': row.read<String>('source'),
              })
          .single;
      history = await probeHistory(db);
      if (phase == 'write') {
        await seedProbe(db, run);
        await evidence.writeAsString(
            jsonEncode({
              'owner': 'dust-sqlite-lifecycle-v1',
              'run': run,
              'pid': pid,
              'history': history,
            }),
            flush: true);
        lifecycle.phase = 'await_background';
        await lifecycle.paused.future.timeout(const Duration(seconds: 60));
        await lifecycle.resumed.future.timeout(const Duration(seconds: 60));
        expect(await probeHistory(db), history);
        report['lifecycle'] = lifecycle.states;
      } else {
        expect(prior!['history'], history);
        report['priorPid'] = prior['pid'];
      }
      await verifyProbe(db, run);
    } finally {
      (await db.close()).unwrap();
    }
    for (var cycle = 0; cycle < 10; cycle++) {
      final reopened = await openProbe(path);
      try {
        expect(await probeHistory(reopened), history);
        await verifyProbe(reopened, run);
      } finally {
        (await reopened.close()).unwrap();
      }
    }
    // Wait for the engine's batched timing delivery, not for SQL readiness.
    await Future<void>.delayed(const Duration(seconds: 1));
    expect(metrics.frames, isNotEmpty);
    report['metrics'] = metrics.report();
    binding.reportData = report;
    lifecycle.phase = 'complete';
    await tester.pumpWidget(const SizedBox.shrink());
    if (phase == 'verify') await directory.delete(recursive: true);
  });
}
