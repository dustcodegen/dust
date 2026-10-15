import 'dart:convert';
import 'dart:io';

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
