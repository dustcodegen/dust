import 'dart:async';
import 'dart:io';

import 'package:flutter_driver/flutter_driver.dart';
import 'package:integration_test/common.dart';
import 'package:integration_test/integration_test_driver.dart';

Future<void> main() async {
  final driver = await FlutterDriver.connect();
  Response? response;
  Object? requestError;
  final completed = driver
      .requestData(
        null,
        timeout: const Duration(minutes: 3),
      )
      .then<void>(
        (value) => response = Response.fromJson(value),
        onError: (Object error) => requestError = error,
      );
  Future<Map<String, dynamic>> waitForPhase(String expected) async {
    final watch = Stopwatch()..start();
    while (watch.elapsed < const Duration(seconds: 60)) {
      if (requestError != null) throw StateError('$requestError');
      if (response != null) {
        throw StateError(
            'Test ended before $expected: ${response!.formattedFailureDetails}');
      }
      final status = await driver.serviceClient
          .callServiceExtension(
            'ext.dust.sqliteProbe',
            isolateId: driver.appIsolate.id,
          )
          .timeout(const Duration(seconds: 5));
      if (status.json!['phase'] == expected) return status.json!;
      await Future<void>.delayed(const Duration(milliseconds: 50));
    }
    throw StateError('Probe did not reach $expected.');
  }

  Future<int> workers() async => (await driver.serviceClient.getVM())
      .isolates!
      .where((isolate) => isolate.name == 'Dust SQLite')
      .length;

  Future<String> adb(List<String> arguments) async {
    final serial = Platform.environment['DUST_SQLITE_ANDROID_SERIAL'];
    final executable = Platform.environment['DUST_SQLITE_ADB'] ?? 'adb';
    if (serial == null || serial.isEmpty) {
      throw StateError(
          'Set DUST_SQLITE_ANDROID_SERIAL to the intended device.');
    }
    final process =
        await Process.start(executable, ['-s', serial, ...arguments]);
    final output =
        process.stdout.transform(const SystemEncoding().decoder).join();
    final errors =
        process.stderr.transform(const SystemEncoding().decoder).join();
    try {
      final code = await process.exitCode.timeout(const Duration(seconds: 20));
      if (code != 0) {
        throw StateError('adb failed: ${await output}\n${await errors}');
      }
      return (await output).trim();
    } on TimeoutException {
      process.kill(ProcessSignal.sigkill);
      await process.exitCode.timeout(const Duration(seconds: 5));
      rethrow;
    }
  }

  try {
    final phase = Platform.environment['DUST_SQLITE_PROBE_PHASE'];
    int? workersDuringPause;
    if (phase == 'write') {
      final ready = await waitForPhase('await_background');
      workersDuringPause = await workers();
      if (workersDuringPause != 1) {
        throw StateError(
            'Expected one live SQLite worker: $workersDuringPause');
      }
      if (Platform.environment.containsKey('DUST_SQLITE_ANDROID_SERIAL')) {
        if (ready['platform'] != 'android' ||
            await adb(['shell', 'pidof', 'com.example.shopping_app']) !=
                '${ready['pid']}') {
          throw StateError(
              'Refusing lifecycle commands on a mismatched device/process.');
        }
        await adb(['shell', 'input', 'keyevent', 'KEYCODE_HOME']);
        await waitForPhase('paused');
        await adb([
          'shell',
          'am',
          'start',
          '-W',
          '-n',
          'com.example.shopping_app/.MainActivity',
        ]);
      } else {
        stdout.writeln(
            'Background the probe, then return to it within 60 seconds.');
      }
    } else if (phase != 'verify') {
      throw StateError('Set DUST_SQLITE_PROBE_PHASE=write or verify.');
    }
    await completed;
    if (requestError != null) throw StateError('$requestError');
    final result = response!;
    if (!result.allTestsPassed) {
      throw StateError(result.formattedFailureDetails);
    }
    final data = result.data;
    if (data == null ||
        data['run'] != Platform.environment['DUST_SQLITE_PROBE_RUN'] ||
        data['phase'] != phase) {
      throw StateError('Missing or mismatched probe report identity.');
    }
    final drain = Stopwatch()..start();
    var remainingWorkers = await workers();
    while (
        remainingWorkers != 0 && drain.elapsed < const Duration(seconds: 5)) {
      await Future<void>.delayed(const Duration(milliseconds: 50));
      remainingWorkers = await workers();
    }
    if (remainingWorkers != 0) {
      throw StateError('SQLite worker survived close: $remainingWorkers');
    }
    data['workersDuringPause'] = workersDuringPause;
    data['workersAfterClose'] = remainingWorkers;
    if (Platform.environment.containsKey('DUST_SQLITE_ANDROID_SERIAL')) {
      data['android'] = {
        'fingerprint': await adb(['shell', 'getprop', 'ro.build.fingerprint']),
        'api': await adb(['shell', 'getprop', 'ro.build.version.sdk']),
        'emulator': await adb(['shell', 'getprop', 'ro.kernel.qemu']) == '1',
      };
    }
    await writeResponseData(
      data,
      testOutputFilename:
          'sqlite-${Platform.environment['DUST_SQLITE_PROBE_RUN']}-$phase',
    );
    stdout.writeln('SQLite $phase phase passed.');
  } finally {
    await driver.close();
  }
}
