import 'package:primary_sdk_consumer/models.dart';
import 'package:test/test.dart';

void main() {
  group('primary generated account', () {
    test('retains defaults, equality and string output', () {
      const account = PrimaryAccount(id: 7, name: 'Ada');
      expect(account.visits, 0);
      expect(account.note, isNull);
      expect(account, const PrimaryAccount(id: 7, name: 'Ada'));
      expect(
        account.hashCode,
        const PrimaryAccount(id: 7, name: 'Ada').hashCode,
      );
      expect(
        account.toString(),
        'PrimaryAccount(id: 7, name: Ada, note: null, visits: 0)',
      );
    });

    test('round-trips renamed keys and constructor defaults', () {
      const account = PrimaryAccount(id: 7, name: 'Ada', note: 'saved');
      expect(account.toJson(), <String, Object?>{
        'account_id': 7,
        'name': 'Ada',
        'note': 'saved',
        'visits': 0,
      });
      expect(PrimaryAccount.fromJson(account.toJson()), account);
      expect(
        PrimaryAccount.fromJson(<String, Object?>{
          'account_id': 7,
          'name': 'Ada',
        }),
        const PrimaryAccount(id: 7, name: 'Ada'),
      );
    });

    test('copyWith distinguishes omission from an explicit null', () {
      const account = PrimaryAccount(id: 7, name: 'Ada', note: 'saved');
      expect(account.copyWith(), account);
      expect(
        account.copyWith(name: 'Grace', note: null),
        const PrimaryAccount(id: 7, name: 'Grace'),
      );
      expect(account.note, 'saved');
    });

    test('field metadata from declaring parameters drives validation', () {
      expect(
        const PrimaryAccount(id: 7, name: 'Ada').validate().isValid,
        isTrue,
      );
      expect(
        const PrimaryAccount(
          id: 7,
          name: '',
        ).validate().errors.map((error) => error.field),
        ['name'],
      );
    });
  });

  test(
    'named primary construction preserves mutation, defaults and copyWith',
    () {
      final counter = PrimaryCounter.named(1)..value = 2;
      expect(counter.value, 2);
      expect(counter.label, 'counter');
      expect(counter.copyWith(value: 3).value, 3);
      expect(counter.toString(), 'PrimaryCounter(value: 2, label: counter)');
    },
  );
}
