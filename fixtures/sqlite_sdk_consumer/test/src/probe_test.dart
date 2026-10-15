import 'package:sqlite_sdk_consumer/src/probe.dart';
import 'package:test/test.dart';

void main() {
  for (final background in [false, true]) {
    test(
      'public ${background ? 'worker' : 'sync'} consumer persists',
      () async {
        final proof = await probe(background: background);
        expect(proof['mode'], background ? 'worker' : 'sync');
        expect(proof['rows'], 1);
        expect(proof['integrity'], 'ok');
        expect(proof['sqliteVersion'], matches(r'^\d+\.\d+\.\d+$'));
        expect(proof['sqliteSourceId'], isNotEmpty);
        expect(
          proof['migrationChecksum'],
          'b5b41cf1bce3408d90653b9b3f15363f49c2cf657c1f714416c4a312f40166aa',
        );
      },
    );
  }
}
