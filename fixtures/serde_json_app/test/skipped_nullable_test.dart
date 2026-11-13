import 'package:serde_json_app/serde_json_app.dart';
import 'package:test/test.dart';

void main() {
  test('nullable skipped fields ignore input and preserve local defaults', () {
    final value = NullableSkipped.fromJson({
      'requiredLocal': <Object?>[],
      'both': false,
      'decodeOnly': 42,
      'optionalDefault': <String, Object?>{},
      'explicitValue': false,
      'initialized': false,
    });

    expect(value.requiredLocal, isNull);
    expect(value.both, isNull);
    expect(value.decodeOnly, isNull);
    expect(value.optionalDefault, 'constructor-default');
    expect(value.explicitValue, 'serde-default');
    expect(value.initialized, 'field-default');
    expect(value.ordinary, isNull);
    expect(value.toJson(), {'decodeOnly': null, 'ordinary': null});
  });

  test('skipDeserializing still serializes the stored value', () {
    final value = NullableSkipped(
      requiredLocal: 'local',
      both: 'hidden',
      decodeOnly: 'visible',
    );
    expect(value.toJson(), {
      'decodeOnly': 'visible',
      'ordinary': 'ordinary-default',
    });
  });

  test('optional positional skipped defaults keep later arguments aligned', () {
    final value = PositionalSkipped.fromJson({'count': 4, 'local': false});
    expect(value.local, 'position-default');
    expect(value.count, 4);
    expect(value.toJson(), {'local': 'position-default', 'count': 4});
  });

  test('strict mode still rejects nullable skipped input keys', () {
    expect(StrictSkipped.fromJson({}).local, isNull);
    expect(const StrictSkipped(local: 'hidden').toJson(), isEmpty);
    expect(
      () => StrictSkipped.fromJson({'local': null}),
      throwsArgumentError,
    );
  });
}
