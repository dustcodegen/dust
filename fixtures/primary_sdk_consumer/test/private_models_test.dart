import 'package:dust_dart/db.dart';
import 'package:primary_sdk_consumer/private_models.dart';
import 'package:test/test.dart';

void main() {
  test(
    'private fields use public copy arguments and retain null semantics',
    () {
      const account = PrivateAccount(name: 'Ada', note: 'saved');
      expect(account.copyWith(), account);
      expect(account.copyWith(name: 'Grace').name, 'Grace');
      expect(account.copyWith(name: 'Grace').note, 'saved');
      expect(account.copyWith(note: null).note, isNull);
      expect(account.note, 'saved');
    },
  );

  test('JSON aliases do not change private field or public argument names', () {
    const account = PrivateAccount(name: 'Ada', note: 'saved');
    expect(account.toJson(), <String, Object?>{
      'display_name': 'Ada',
      'note': 'saved',
    });
    expect(PrivateAccount.fromJson(account.toJson()), account);
  });

  test('row mapping invokes private declaring fields with public labels', () {
    final account = const $PrivateAccountRowDeserializer().deserialize(
      const _MapRow(<String, Object?>{'display_name': 'Ada', 'note': null}),
    );
    expect(account, const PrivateAccount(name: 'Ada'));
  });

  test('nested copyWith uses the public path and preserves the original', () {
    const account = PrivateAccount(name: 'Ada', note: 'saved');
    const envelope = PrivateEnvelope(account: account);
    expect(envelope.copyWith.account!(name: 'Grace').account!.name, 'Grace');
    expect(envelope.copyWith.account!(note: null).account!.note, isNull);
    expect(envelope.copyWith(account: null).account, isNull);
    expect(const PrivateEnvelope().copyWith.account, isNull);
    expect(envelope.account, account);
  });

  test('private positional fields do not capture generated copy locals', () {
    const model = PrivateLocals('original', 'callback', 'value');
    expect(model.copyWith(self: 'new').values, ('new', 'callback', 'value'));
    expect(model.copyWith(then: 'new').values, ('original', 'new', 'value'));
    expect(model.copyWith(value: 'new').values, (
      'original',
      'callback',
      'new',
    ));
    expect(model.copyWith().values, model.values);
  });

  test('non-nullable private nested fields expose a public copy path', () {
    const owner = PrivateOwner(account: PrivateAccount(name: 'Ada'));
    expect(owner.copyWith.account(name: 'Grace').account.name, 'Grace');
    expect(owner.account.name, 'Ada');
  });

  test(
    'traditional private initializing formals use the same public labels',
    () {
      const model = TraditionalPrivate(name: 'Ada');
      expect(model.copyWith(name: 'Grace').name, 'Grace');
      expect(TraditionalPrivate.fromJson(model.toJson()).name, 'Ada');
    },
  );
}

final class _MapRow implements Row {
  const _MapRow(this.values);

  final Map<String, Object?> values;

  @override
  T read<T>(String column) => values[column] as T;

  @override
  T? readNullable<T>(String column) => values[column] as T?;

  @override
  T readIndex<T>(int index) => throw UnsupportedError('unused index read');

  @override
  T? readIndexNullable<T>(int index) =>
      throw UnsupportedError('unused index read');

  @override
  bool readBool(String column) => throw UnsupportedError('unused boolean read');

  @override
  bool? readBoolNullable(String column) =>
      throw UnsupportedError('unused boolean read');

  @override
  DateTime readDateTime(String column) =>
      throw UnsupportedError('unused datetime read');

  @override
  DateTime? readDateTimeNullable(String column) =>
      throw UnsupportedError('unused datetime read');
}
