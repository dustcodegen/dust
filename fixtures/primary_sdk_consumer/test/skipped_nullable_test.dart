import 'package:primary_sdk_consumer/skipped_nullable.dart';
import 'package:test/test.dart';

void main() {
  test('primary skipped fields keep nullable and declared defaults', () {
    final value = PrimarySkipped.fromJson({
      'requiredLocal': false,
      'local': 42,
      'cached': <Object?>[],
    });
    expect(value.requiredLocal, isNull);
    expect(value.local, isNull);
    expect(value.cached, 'primary-default');
    expect(value.toJson(), {'cached': 'primary-default'});
  });
}
