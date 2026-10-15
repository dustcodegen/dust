import 'dart:async';

import 'package:test/test.dart';

import '../example/wal_checkpoint.dart' as example;

void main() {
  for (final args in [
    <String>[],
    ['--background'],
  ]) {
    test('WAL checkpoint example $args', () async {
      final lines = <String>[];
      await runZoned(
        () => example.main(args),
        zoneSpecification: ZoneSpecification(
          print: (self, parent, zone, line) => lines.add(line),
        ),
      );
      expect(lines, [
        'reader snapshot: 1, checkpoint busy: 1',
        'checkpoint complete: (0, 0, 0), current value: 2',
      ]);
    });
  }
}
