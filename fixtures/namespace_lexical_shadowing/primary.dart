// @dart=3.13
import 'dart:mirrors';

import 'annotations.dart' as d;

@d.Marker()
class Model(final int value) {
  static const d = Object();

  @d
  void call(@d int other) {}
}

void main() {
  final method = reflectClass(Model).declarations[#call]! as MethodMirror;
  assert(identical(method.metadata.single.reflectee, Model.d));
  assert(
    identical(method.parameters.single.metadata.single.reflectee, Model.d),
  );
  assert(Model(7).value == 7);
}
