import 'dart:async';

import 'package:test/test.dart';

import '../example/migration_commit_failure.dart' as example;

void main() {
  for (final args in [
    <String>[],
    ['--background'],
  ]) {
    test('migration COMMIT example $args', () async {
      final lines = <String>[];
      await runZoned(
        () => example.main(args),
        zoneSpecification: ZoneSpecification(
          print: (self, parent, zone, line) => lines.add(line),
        ),
      );
      expect(lines, [
        'migration failed at: COMMIT',
        'dependency repaired: rows: 1',
      ]);
    });
  }
}
