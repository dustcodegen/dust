import 'dart:io';

import 'package:test/test.dart';

import 'support/bounded_volume.dart';
import 'support/filesystem_process.dart';
import 'support/native_process.dart';

void main() {
  late Directory binaries;
  late String child;
  setUpAll(() async {
    if (!Platform.isMacOS) {
      throw UnsupportedError('The bounded disk-image gate requires macOS.');
    }
    binaries = Directory.systemTemp.createTempSync('dust-filesystem-child-');
    addTearDown(() => binaries.deleteSync(recursive: true));
    child = await buildNativeChild(
      binaries,
      'integration_test/support/filesystem_child.dart',
    );
  });
  test('bounded volume acquisition and verified detach', () async {
    final volume = await BoundedVolume.create();
    try {
      expect(volume.uuid, isNotEmpty);
      await volume.verify();
    } finally {
      await volume.close();
    }
    expect(volume.directory.existsSync(), isFalse);
  });
  for (final journal in ['delete', 'wal']) {
    for (final mode in ['sync', 'worker']) {
      for (final stage in [
        'transaction',
        'nested',
        'ignored',
        'standalone',
        'migration',
        if (journal == 'wal') 'checkpoint',
      ]) {
        test('$journal / $mode / $stage / real ENOSPC', () async {
          final volume = await BoundedVolume.create();
          try {
            await filesystemProcess(child, volume, journal, mode, stage);
          } finally {
            await volume.close();
          }
          expect(volume.directory.existsSync(), isFalse);
        });
      }
    }
  }
}
