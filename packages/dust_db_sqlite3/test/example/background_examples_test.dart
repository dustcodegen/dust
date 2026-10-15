import 'package:test/test.dart';

import '../../example/background_commit_recovery.dart' as recovery;
import '../../example/open_in_background.dart' as opening;
import 'capture.dart';

void main() {
  test('open_in_background', () async {
    expect(await runExample(opening.main), [
      'notes: [Opened in the worker]',
    ]);
  });
  test('background_commit_recovery', () async {
    expect(await runExample(recovery.main), [
      'failed at: COMMIT',
      'children after rollback: 0',
      'next transaction committed: true',
    ]);
  });
}
