import 'dart:convert';
import 'dart:io';

import 'package:test/test.dart';

import 'bounded_volume.dart';

Future<void> filesystemProcess(
  String executable,
  BoundedVolume volume,
  String journal,
  String mode,
  String stage,
) async {
  final marker = '${volume.directory.path}/child';
  final path = '${volume.mountPoint}/app.db';
  final process = await Process.start(executable, [
    path,
    journal,
    mode,
    stage,
    marker,
  ]);
  final output = process.stdout.transform(utf8.decoder).join();
  final errors = process.stderr.transform(utf8.decoder).join();
  int? code;
  final exited = process.exitCode.then((value) => code = value);
  Future<void> waitFor(String phase) async {
    final deadline = Stopwatch()..start();
    while (!File('$marker.$phase').existsSync() && code == null) {
      if (deadline.elapsed > const Duration(seconds: 20)) {
        fail('child handshake timed out: $phase');
      }
      await Future<void>.delayed(const Duration(milliseconds: 10));
    }
    expect(
      code,
      isNull,
      reason: 'child exited before $phase: ${code == null ? '' : await errors}',
    );
  }

  try {
    await waitFor('ready');
    expect(jsonDecode(File('$marker.ready').readAsStringSync()), 'ready');
    if (stage == 'checkpoint') {
      expect(File('$path-wal').lengthSync(), greaterThan(262144));
      expect(File(path).lengthSync(), lessThan(262144));
    }
    final filled = await volume.fill();
    expect(filled.bytes, inInclusiveRange(1, volumeLimit));
    expect(filled.errno, 28);
    process.stdin.writeln();
    await process.stdin.flush();
    await waitFor('failed');
    final failure =
        jsonDecode(File('$marker.failed').readAsStringSync())
            as Map<String, Object?>;
    expect(failure['nativeCode'], 13);
    await volume.restoreSpace();
    process.stdin.writeln();
    await process.stdin.flush();
    await exited.timeout(const Duration(seconds: 20));
    expect(code, 0, reason: '${await output}\n${await errors}');
    final proof = jsonDecode(await output) as Map<String, Object?>;
    expect(proof['status'], 'ok');
    expect(proof['stage'], stage);
    expect(proof['engine'], isA<String>());
    expect(proof['sourceId'], isNotEmpty);
    expect(await errors, isEmpty);
    print(
      jsonEncode({
        'journal': journal,
        'mode': mode,
        'stage': stage,
        'filesystem': 'hfs',
        'device': volume.device,
        'uuid': volume.uuid,
        'capacity': volume.capacity,
        'fillerBytes': filled.bytes,
        'osError': filled.errno,
        'sqliteCode': failure['nativeCode'],
        'engine': proof['engine'],
        'sourceId': proof['sourceId'],
      }),
    );
  } finally {
    if (code == null) process.kill(ProcessSignal.sigkill);
    await exited.timeout(const Duration(seconds: 10));
    await process.stdin.close();
  }
}
