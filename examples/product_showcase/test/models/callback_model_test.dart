import 'package:product_showcase/models/callback_model.dart';
import 'package:test/test.dart';

T _identity<T>(T value) => value;

void main() {
  group('explicit generic callback copies', () {
    test('omission and non-nullable null preserve the original callback', () {
      const model = CallbackModel(transform: _identity);
      final omitted = model.copyWith();
      final nullReplacement = model.copyWith(transform: null);

      expect(omitted, isNot(same(model)));
      expect(omitted.transform, same(model.transform));
      expect(nullReplacement.transform, same(model.transform));
      final integer = omitted.transform<int>(42);
      final text = nullReplacement.transform<String>('dust');
      expect(integer, 42);
      expect(text, 'dust');
    });

    test('a replacement remains callable at different type arguments', () {
      var calls = 0;
      T replacement<T>(T value) {
        calls++;
        return value;
      }

      const model = CallbackModel(transform: _identity);
      final copied = model.copyWith(transform: replacement);
      final integer = copied.transform<int>(7);
      final text = copied.transform<String>('updated');
      expect(integer, 7);
      expect(text, 'updated');
      expect(calls, 2);
      expect(model.transform, same(_identity));
    });

    test('nullable callbacks distinguish omission, replacement and clearing',
        () {
      const model = CallbackModel(
        transform: _identity,
        optionalTransform: _identity,
      );
      final omitted = model.copyWith();
      final cleared = model.copyWith(optionalTransform: null);
      final replaced = cleared.copyWith(optionalTransform: _identity);

      expect(omitted.optionalTransform, same(model.optionalTransform));
      expect(cleared.optionalTransform, isNull);
      expect(replaced.optionalTransform, same(_identity));
      final text = replaced.optionalTransform!<String>('restored');
      expect(text, 'restored');
    });
  });
}
