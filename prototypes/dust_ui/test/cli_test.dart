import 'dart:io';

import 'package:test/test.dart';
import 'package:test_process/test_process.dart';

void main() {
  test('help lists both commands', () async {
    final process = await TestProcess.start('dart', [
      'run',
      'bin/dust_ui.dart',
      '--help',
    ]);
    expect(await process.stdout.next, 'Usage:');
    expect(await process.stdout.next, contains('check [--fragment] FILE'));
    expect(await process.stdout.next, contains('preview'));
    await process.shouldExit(0);
  });

  test(
    'check accepts the demo and reports file, line and column for errors',
    () async {
      final scratch = await Directory.systemTemp.createTemp('dust-ui-cli');
      addTearDown(() => scratch.delete(recursive: true));
      final broken = await File.fromUri(
        scratch.uri.resolve('broken.dustui'),
      ).writeAsString('col {\n  nonexistent()\n}\n');
      final process = await TestProcess.start('dart', [
        'run',
        'bin/dust_ui.dart',
        'check',
        'demo/product.dustui',
        broken.path,
      ]);
      expect(await process.stdout.next, 'demo/product.dustui: OK');
      expect(await process.stderr.next, startsWith('${broken.path}:2:3:'));
      await process.shouldExit(1);
    },
  );

  test('fragment mode allows targets outside the supplied document', () async {
    final scratch = await Directory.systemTemp.createTemp('dust-ui-fragment');
    addTearDown(() => scratch.delete(recursive: true));
    final file = await File.fromUri(scratch.uri.resolve('region.dustui'))
        .writeAsString(
          'button(get: "/ui/cart", target: "cart") { text(value: "Refresh") }',
        );
    final screen = await TestProcess.start('dart', [
      'run',
      'bin/dust_ui.dart',
      'check',
      file.path,
    ]);
    expect(await screen.stderr.next, contains('cart'));
    await screen.shouldExit(1);
    final fragment = await TestProcess.start('dart', [
      'run',
      'bin/dust_ui.dart',
      'check',
      '--fragment',
      file.path,
    ]);
    expect(await fragment.stdout.next, '${file.path}: OK');
    await fragment.shouldExit(0);
  });

  test('missing files on the command line are a usage error', () async {
    final process = await TestProcess.start('dart', [
      'run',
      'bin/dust_ui.dart',
      'check',
    ]);
    expect(await process.stderr.next, contains('Pass at least one'));
    await process.shouldExit(64);
  });
}
