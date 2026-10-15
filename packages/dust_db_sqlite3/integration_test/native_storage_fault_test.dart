import 'dart:convert';
import 'dart:io';

import 'package:test/test.dart';

import 'support/native_process.dart';

void main() {
  late Directory binaries;
  late String extension;
  late String child;
  setUpAll(() async {
    if (!Platform.isLinux && !Platform.isMacOS) {
      throw UnsupportedError(
        'This native VFS harness requires macOS or Linux; it never silently skips.',
      );
    }
    binaries = Directory.systemTemp.createTempSync('dust-native-io-');
    addTearDown(() => binaries.deleteSync(recursive: true));
    extension =
        '${binaries.path}/fault.${Platform.isWindows
            ? 'dll'
            : Platform.isMacOS
            ? 'dylib'
            : 'so'}';
    child = '${binaries.path}/child${Platform.isWindows ? '.exe' : ''}';
    final built = await nativeProcess('clang', [
      '-std=c11',
      '-Wall',
      '-Wextra',
      '-Werror',
      if (!Platform.isWindows) '-fPIC',
      Platform.isMacOS ? '-dynamiclib' : '-shared',
      'integration_test/support/fault_vfs.c',
      '-lsqlite3',
      '-o',
      extension,
    ]);
    expect(built.exitCode, 0, reason: '${built.stdout}\n${built.stderr}');
    final compiled = await nativeProcess(Platform.resolvedExecutable, [
      'compile',
      'exe',
      'integration_test/support/native_fault_child.dart',
      '-o',
      child,
    ]);
    expect(
      compiled.exitCode,
      0,
      reason: '${compiled.stdout}\n${compiled.stderr}',
    );
  });

  Future<void> scenario(
    String journal,
    String mode,
    String stage,
    String operation,
    String target,
    String persistence,
    String writeKind,
  ) async {
    final directory = Directory.systemTemp.createTempSync('dust-native-case-');
    addTearDown(() => directory.deleteSync(recursive: true));
    final result = await nativeProcess(
      child,
      [
        extension,
        '${directory.path}/app.db',
        journal,
        mode,
        stage,
        operation,
        target,
        persistence,
        writeKind,
      ],
      environment: {
        'DUST_TEST_VFS_CONTROL': '${directory.path}/app.db.control',
      },
    );
    expect(result.exitCode, 0, reason: '${result.stdout}\n${result.stderr}');
    expect(result.stderr, isEmpty);
    final output = jsonDecode(result.stdout as String) as Map<String, Object?>;
    expect(output['status'], 'ok');
    expect(output['hits'], greaterThan(0));
    if (persistence == 'once') expect(output['hits'], 1);
    expect(
      output['partialWrites'],
      writeKind == 'partial' ? greaterThan(0) : 0,
    );
    expect(output['partialBytes'], writeKind == 'partial' ? greaterThan(0) : 0);
    expect(output['engine'], isA<String>());
    expect(output['sourceId'], isNotEmpty);
  }

  for (final mode in ['sync', 'worker']) {
    for (final persistence in ['once', 'persistent']) {
      for (final (journal, target) in [
        ('delete', 'journal'),
        ('delete', 'database'),
        ('wal', 'wal'),
      ]) {
        for (final (operation, writeKind) in [
          ('write', 'whole'),
          ('write', 'partial'),
          ('sync', 'whole'),
        ]) {
          for (final stage in ['transaction', 'nested', 'migration']) {
            test(
              '$journal / $mode / $stage / $target $operation $writeKind / $persistence',
              () async => scenario(
                journal,
                mode,
                stage,
                operation,
                target,
                persistence,
                writeKind,
              ),
            );
          }
        }
      }
      for (final (operation, target, writeKind) in [
        ('write', 'database', 'whole'),
        ('write', 'database', 'partial'),
        ('sync', 'database', 'whole'),
        ('truncate', 'wal', 'whole'),
      ]) {
        test(
          'wal / $mode / checkpoint / $target $operation $writeKind / $persistence',
          () async => scenario(
            'wal',
            mode,
            'checkpoint',
            operation,
            target,
            persistence,
            writeKind,
          ),
        );
      }
      for (final journal in ['delete', 'wal']) {
        test(
          '$journal / $mode / read / $persistence',
          () async => scenario(
            journal,
            mode,
            'read',
            'read',
            'database',
            persistence,
            'whole',
          ),
        );
      }
    }
  }
}
