import 'package:primary_sdk_consumer/generic_models.dart';
import 'package:primary_sdk_consumer/generic_imported.dart';
import 'package:test/test.dart';

void main() {
  group('generated generic copies', () {
    test('retain the chosen result type and normal instance dispatch', () {
      final original = GenericBox<int>(1);
      final GenericBox<int> Function() omitted = original.copyWith.call;
      expect(omitted().value, 1);
      expect(original.copyWith(value: 2).value, 2);
      expect(original.copyWith(value: null).value, 1);
      expect(original.copyWith(), original);
      expect(original, isNot(GenericBox<num>(1)));
      expect(original.hashCode, original.copyWith().hashCode);
      expect(original.toString(), 'GenericBox(value: 1)');
      final dynamic receiver = original;
      final dynamic copied = receiver.copyWith(value: 3);
      expect(copied, isA<GenericBox<int>>());
      expect(copied.value, 3);
      expect(
        () => receiver.copyWith(value: 'wrong'),
        throwsA(isA<TypeError>()),
      );
    });

    test(
      'distinguish omission and null for nullable, dynamic and Null values',
      () {
        const nullable = GenericBox<String?>('before');
        expect(nullable.copyWith().value, 'before');
        expect(nullable.copyWith(value: null).value, isNull);
        expect(nullable.copyWith(value: 'after').value, 'after');
        const object = Object();
        final dynamicBox = GenericBox<dynamic>(object);
        expect(dynamicBox.copyWith().value, same(object));
        expect(dynamicBox.copyWith(value: null).value, isNull);
        const objectBox = GenericBox<Object?>('before');
        expect(objectBox.copyWith().value, 'before');
        expect(objectBox.copyWith(value: object).value, same(object));
        expect(objectBox.copyWith(value: null).value, isNull);
        expect(const GenericBox<Null>(null).copyWith().value, isNull);
      },
    );

    test('preserve bounds and named constructors', () {
      const original = BoundedBox<num>.named(1);
      final BoundedBox<num> Function() copy = original.copyWith.call;
      expect(copy().value, 1);
      expect(original.copyWith(value: 2.5).value, 2.5);
      expect(original.copyWith(value: null).value, 1);
    });

    test('respect outer containers, FutureOr and nullable aliases', () async {
      final values = <String?>['before', null];
      final original = GenericValues<String?>(
        values: values,
        pending: 'before',
        maybe: 'before',
      );
      expect(original.copyWith().values, same(values));
      expect(original.copyWith(values: null).values, same(values));
      expect(original.copyWith().pending, 'before');
      expect(original.copyWith(pending: null).pending, isNull);
      expect(original.copyWith().maybe, 'before');
      expect(original.copyWith(maybe: null).maybe, isNull);
      final future = Future<String?>.value('future');
      expect(original.copyWith(pending: future).pending, same(future));
      expect(await original.copyWith(pending: future).pending, 'future');
      const nonNullable = GenericValues<int>(values: [1, null], pending: 1);
      expect(nonNullable.copyWith(pending: null).pending, 1);
      expect(
        nonNullable.copyWith(maybe: 7).copyWith(maybe: null).maybe,
        isNull,
      );
    });

    test('preserve callback binders and record member types', () {
      String? choose<U>(String? value, U _) => value;
      String? replacement<U>(String? value, U _) => '$value updated';
      final original = GenericShapes<String?>(
        row: (item: 'before', count: 1),
        choose: choose,
      );
      final copied = original.copyWith(row: (item: null, count: 2));
      expect(copied.row.item, isNull);
      expect(copied.row.count, 2);
      expect(copied.choose<int>('value', 7), 'value');
      expect(original.copyWith(row: null).row, original.row);
      expect(original.copyWith(choose: null).choose, same(choose));
      expect(
        original.copyWith(choose: replacement).choose<int>('value', 1),
        'value updated',
      );
    });

    test('instantiate nested helpers with actual child arguments', () {
      const original = GenericEnvelope<int>(
        child: GenericBox(1),
        optional: GenericBox(2),
      );
      final updated = original.copyWith.child(value: 3);
      expect(updated.child.value, 3);
      expect(updated.optional, same(original.optional));
      expect(original.copyWith.optional!(value: 4).optional!.value, 4);
      expect(original.copyWith(optional: null).copyWith.optional, isNull);
      expect(original.copyWith(child: null).child, same(original.child));
      expect(original.copyWith.child(value: null).child.value, 1);
      expect(original.copyWith(child: GenericBox(9)).child.value, 9);
    });

    test('avoid the result binder collision and retain phantom arguments', () {
      const original = ResultNamed<String?>('before');
      final ResultNamed<String?> Function() copy = original.copyWith.call;
      expect(copy().value, 'before');
      expect(original.copyWith(value: null).value, isNull);
      final empty = const EmptyBox<int>().copyWith.call;
      expect(empty(), isA<EmptyBox<int>>());
    });

    test(
      'forward dependent and recursive bounds through helper declarations',
      () {
        final original = DependentBox<int, List<int>>(1, [1]);
        final DependentBox<int, List<int>> Function() copy =
            original.copyWith.call;
        expect(copy().value, 1);
        expect(original.copyWith(value: 2, values: [2]).values, [2]);
        expect(
          original.copyWith(value: null, values: null).values,
          same(original.values),
        );
        const recursive = RecursiveBox<String>('before');
        final RecursiveBox<String> Function() copyRecursive =
            recursive.copyWith.call;
        expect(copyRecursive().value, 'before');
        expect(recursive.copyWith(value: 'after').value, 'after');
        expect(recursive.copyWith(value: null).value, 'before');
      },
    );

    test(
      'preserve compound nested arguments and imported replacement types',
      () {
        const original = GenericListEnvelope<int>(GenericBox([1, null]));
        expect(original.copyWith().child, same(original.child));
        expect(original.copyWith(child: null).child, same(original.child));
        final replacement = <int?>[2, null];
        final updated = original.copyWith.child(value: replacement);
        expect(updated.child.value, [2, null]);
        expect(updated.child.value, same(replacement));
        expect(updated, isA<GenericListEnvelope<int>>());
        const imported = ImportedEnvelope<int>(GenericBox(1));
        final ImportedEnvelope<int> Function() copy = imported.copyWith.call;
        expect(copy().child.value, 1);
        expect(imported.copyWith(child: null).child, same(imported.child));
        expect(imported.copyWith(child: GenericBox(2)).child.value, 2);
      },
    );
  });
}
