import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:test/test.dart';

Future<({int code, String out, String err})> _run(
  List<String> arguments,
) async {
  final process = await Process.start(Platform.resolvedExecutable, [
    'run',
    '--verbosity=error',
    'bin/sqlite_sdk_consumer.dart',
    ...arguments,
  ]);
  final out = process.stdout.transform(utf8.decoder).join();
  final err = process.stderr.transform(utf8.decoder).join();
  try {
    return await (() async => (
      code: await process.exitCode,
      // Dart 3.10 writes this compiler progress to stdout, even at error verbosity.
      out: (await out).replaceFirst(
        RegExp(r'^(?:Running build hooks\.\.\.)+'),
        '',
      ),
      err: await err,
    ))().timeout(const Duration(seconds: 60));
  } finally {
    process.kill(ProcessSignal.sigkill);
    await process.exitCode;
  }
}

void main() {
  test(
    'help documents source and bundle invocation without running SQL',
    () async {
      final result = await _run(['--help']);
      expect(result.code, 0);
      expect(result.err, isEmpty);
      expect(result.out, contains('dart run bin/sqlite_sdk_consumer.dart'));
      expect(result.out, contains('bundled bin/sqlite_sdk_consumer'));
      expect(result.out, isNot(contains('sqliteVersion')));
    },
  );

  for (final arguments in [
    ['--unknown'],
    ['unexpected'],
    ['--help', 'unexpected'],
  ]) {
    test('invalid $arguments exits with usage status', () async {
      final result = await _run(arguments);
      expect(result.code, 64);
      expect(result.out, isEmpty);
      expect(result.err, contains('Usage:'));
    });
  }

  test('CLI returns sync and worker proof and exits cleanly', () async {
    final result = await _run([]);
    expect(result.code, 0, reason: result.err);
    expect(result.err, isEmpty);
    final proof = jsonDecode(result.out) as List<Object?>;
    expect(proof, hasLength(2));
    expect((proof[0]! as Map<String, Object?>)['mode'], 'sync');
    expect((proof[1]! as Map<String, Object?>)['mode'], 'worker');
  });
}
