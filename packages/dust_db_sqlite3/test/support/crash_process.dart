import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:test/test.dart';

/// Kills only this harness's child, after its explicit on-disk readiness marker.
Future<void> crashProcess(
  String executable,
  String path,
  String journal,
  String stage, {
  required bool background,
}) async {
  final marker = File('$path.ready');
  final process = await Process.start(executable, [
    path,
    marker.path,
    journal,
    background ? 'worker' : 'sync',
    stage,
  ]);
  final output = process.stdout.transform(utf8.decoder).join();
  final errors = process.stderr.transform(utf8.decoder).join();
  int? exitCode;
  final exited = process.exitCode.then((code) => exitCode = code);
  final signal = Platform.isWindows
      ? ProcessSignal.sigterm
      : ProcessSignal.sigkill;
  try {
    final deadline = Stopwatch()..start();
    while (!marker.existsSync() && exitCode == null) {
      if (deadline.elapsed > const Duration(seconds: 10)) {
        fail('child readiness timed out: $stage');
      }
      // Poll an explicit handshake, never guess when SQL has finished.
      await Future<void>.delayed(const Duration(milliseconds: 10));
    }
    expect(exitCode, isNull, reason: 'child exited before $stage');
    expect(marker.readAsStringSync(), 'ready');
    if (journal == 'wal' || !stage.endsWith('committed')) {
      final journalFile = File(
        '$path${journal == 'wal' ? '-wal' : '-journal'}',
      );
      expect(journalFile.existsSync(), isTrue);
      expect(
        journalFile.lengthSync(),
        greaterThan(journal == 'wal' ? 65536 : 512),
      );
    }
    if (journal == 'delete' &&
        stage != 'committed' &&
        stage != 'migration-committed') {
      expect(File(path).lengthSync(), greaterThan(65536));
    }
    expect(process.kill(signal), isTrue);
    await exited.timeout(const Duration(seconds: 10));
    expect(exitCode, isNot(0));
    if (!Platform.isWindows) {
      expect(exitCode, -ProcessSignal.sigkill.signalNumber);
    }
  } finally {
    if (exitCode == null) process.kill(signal);
    await exited.timeout(const Duration(seconds: 10));
    await process.stdin.close();
    final stdout = await output;
    final stderr = await errors;
    expect(stderr, isEmpty, reason: 'child stdout: $stdout');
  }
}
