import 'dart:convert';
import 'dart:io';

/// Keep code-asset metadata and libraries beside each recovery test child.
Future<String> buildNativeChild(Directory output, String source) async {
  // CLI builds replace their output directory; keep sibling native shims intact.
  final bundleRoot = '${output.path}/native';
  final built = await nativeProcess(Platform.resolvedExecutable, [
    'build',
    'cli',
    '--output=$bundleRoot',
    '--target=$source',
  ]);
  if (built.exitCode != 0) {
    throw StateError(
      'Native child build failed: ${built.stdout}\n${built.stderr}',
    );
  }
  final file = File(source).uri.pathSegments.last;
  final name = file.substring(0, file.length - '.dart'.length);
  final executable =
      '$bundleRoot/bundle/bin/$name${Platform.isWindows ? '.exe' : ''}';
  if (!File(executable).existsSync()) {
    throw StateError('Native child build produced no executable: $executable');
  }
  return executable;
}

/// Own every spawned process, including timeout/failure cleanup.
Future<ProcessResult> nativeProcess(
  String executable,
  List<String> args, {
  Map<String, String>? environment,
}) async {
  final process = await Process.start(
    executable,
    args,
    environment: environment,
  );
  final output = process.stdout.transform(utf8.decoder).join();
  final errors = process.stderr.transform(utf8.decoder).join();
  int? code;
  final exited = process.exitCode.then((value) => code = value);
  try {
    await exited.timeout(const Duration(seconds: 30));
  } finally {
    if (code == null) {
      process.kill(
        Platform.isWindows ? ProcessSignal.sigterm : ProcessSignal.sigkill,
      );
    }
    await exited.timeout(const Duration(seconds: 10));
    await process.stdin.close();
  }
  return ProcessResult(process.pid, code!, await output, await errors);
}
