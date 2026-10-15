import 'dart:convert';
import 'dart:io';

import 'package:args/args.dart';
import 'package:sqlite_sdk_consumer/src/probe.dart';

Future<void> main(List<String> arguments) async {
  final parser = ArgParser()
    ..addFlag('help', abbr: 'h', negatable: false, help: 'Show usage.');
  const usage =
      'Usage: dart run bin/sqlite_sdk_consumer.dart [--help]\n'
      'Or run the bundled bin/sqlite_sdk_consumer executable by its path.\n'
      'Without flags, verifies sync and worker SQLite and prints JSON proof.';
  try {
    final results = parser.parse(arguments);
    if (results.rest.isNotEmpty) {
      throw FormatException('Unexpected arguments: ${results.rest.join(' ')}');
    }
    if (results.flag('help')) {
      stdout.writeln('$usage\n${parser.usage}');
      return;
    }
  } on FormatException catch (error) {
    stderr.writeln('${error.message}\n$usage\n${parser.usage}');
    exitCode = 64;
    return;
  }
  stdout.writeln(
    jsonEncode([
      await probe(background: false),
      await probe(background: true),
    ]),
  );
}
