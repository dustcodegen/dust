import 'package:primary_sdk_consumer/language_models.dart';
import 'package:test/test.dart';

void main() {
  test('primary enums retain induced fields and generated variant aliases', () {
    expect(PrimaryStatus.active.label, 'Active');
    expect(PrimaryStatus.inactive.label, 'Inactive');
    const record = StatusRecord(id: 7);
    expect(record.toJson(), <String, Object?>{'id': 7, 'status': 'enabled'});
    expect(StatusRecord.fromJson(record.toJson()), record);
    expect(
      record.copyWith(status: PrimaryStatus.inactive).status,
      PrimaryStatus.inactive,
    );
  });

  test(
    'extension representations and trivial mixin primary headers coexist',
    () {
      const id = RecordId(7);
      expect(id.value, 7);
      expect(
        const TaggedRecord('saved').toString(),
        'TaggedRecord(tag: saved)',
      );
    },
  );
}
