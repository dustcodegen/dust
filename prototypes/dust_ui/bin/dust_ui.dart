import 'dart:io';

import 'package:args/args.dart';
import 'package:dust_ui_prototype/language.dart';
import 'package:dust_ui_prototype/src/dev/preview_server.dart';

ArgParser _parser() =>
    ArgParser()..addFlag('help', abbr: 'h', negatable: false);

final _commands = _parser()
  ..addCommand(
    'check',
    _parser()..addFlag(
      'fragment',
      negatable: false,
      help: 'Allow targets resolved by the containing screen.',
    ),
  )
  ..addCommand(
    'preview',
    _parser()
      ..addOption(
        'source',
        help: 'Edit this source instead of the demo product.',
      )
      ..addOption(
        'port',
        defaultsTo: '0',
        help: 'Loopback port (0 picks a free port).',
      ),
  );

Future<void> main(List<String> args) async {
  try {
    final options = _commands.parse(args);
    final command = options.command;
    if (options.flag('help') ||
        (command?.flag('help') ?? false) ||
        (command == null && options.rest.isEmpty)) {
      stdout.writeln(_usage);
      return;
    }
    if (options.rest.isNotEmpty || command == null) {
      throw const FormatException('Expected a command.');
    }
    switch (command.name) {
      case 'check':
        if (command.rest.isEmpty) {
          throw const FormatException('Pass at least one .dustui file.');
        }
        for (final path in command.rest) {
          await _check(File(path), fragment: command.flag('fragment'));
        }
      case 'preview':
        await _preview(command);
    }
  } on FormatException catch (error) {
    stderr.writeln(error.message);
    stderr.writeln(_usage);
    exitCode = 64;
  } on FileSystemException catch (error) {
    stderr.writeln(error);
    exitCode = 1;
  } on ProcessException catch (error) {
    stderr.writeln(error);
    exitCode = 1;
  } on SocketException catch (error) {
    stderr.writeln(error.message);
    exitCode = 1;
  }
}

Future<void> _check(File file, {required bool fragment}) async {
  try {
    if (await file.length() > 128 * 1024) {
      throw const FormatException('Source is larger than 128 KiB.');
    }
    parseUi(await file.readAsString(), fragment: fragment);
    stdout.writeln('${file.path}: OK');
  } on UiSyntaxException catch (error) {
    stderr.writeln(
      '${file.path}:${error.line}:${error.column}: ${error.message}',
    );
    exitCode = 1;
  } on FileSystemException catch (error) {
    stderr.writeln('${file.path}: ${error.message}');
    exitCode = 1;
  } on FormatException catch (error) {
    stderr.writeln('${file.path}: ${error.message}');
    exitCode = 1;
  }
}

Future<void> _preview(ArgResults command) async {
  final port = int.tryParse(command.option('port')!);
  if (command.rest.isNotEmpty || port == null || port < 0 || port > 65535) {
    throw const FormatException(
      'Use preview [--source FILE] [--port 0–65535].',
    );
  }
  final sourcePath = command.option('source');
  final source = sourcePath == null ? null : File(sourcePath).absolute;
  if (source != null && !await source.exists()) {
    throw FileSystemException('Source file does not exist', source.path);
  }
  final root = File.fromUri(Platform.script).parent.parent;
  final build = await Process.start(
    'flutter',
    ['build', 'web', '--no-web-resources-cdn'],
    workingDirectory: root.path,
    mode: ProcessStartMode.inheritStdio,
  );
  final buildExit = await build.exitCode;
  if (buildExit != 0) {
    exitCode = buildExit;
    return;
  }
  final server = await startPreviewServer(
    bundle: Directory.fromUri(root.uri.resolve('build/web/')),
    demo: Directory.fromUri(root.uri.resolve('demo/')),
    port: port,
    source: source,
  );
  stdout.writeln('Dust UI Lab  http://127.0.0.1:${server.port}/');
}

const _usage = '''
Usage:
  dart run bin/dust_ui.dart check [--fragment] FILE [FILE ...]
  dart run bin/dust_ui.dart preview [--source FILE] [--port PORT]
''';
