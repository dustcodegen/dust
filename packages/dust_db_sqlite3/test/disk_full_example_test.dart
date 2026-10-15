import 'dart:async';

import 'package:test/test.dart';

import '../example/disk_full_recovery.dart' as example;

void main() {
  for (final args in [
    <String>[],
    ['--background'],
  ]) {
    test('disk-full example $args', () async {
      final lines = <String>[];
      await runZoned(
        () => example.main(args),
        zoneSpecification: ZoneSpecification(
          print: (self, parent, zone, line) => lines.add(line),
        ),
      );
      expect(lines, [
        'page limit: write failed, rows: 0',
        'capacity restored: rows: 1',
      ]);
    });
  }
}
