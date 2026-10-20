import 'dart:mirrors';

import 'annotations.dart';
import 'annotations.dart' as d;

@d.Marker()
class Model {
  @d
  const Model(@d this.value);

  static const d = Object();

  @d
  final int value;

  @d
  void call(@d int other) {}
}

class Api {
  static const GET = Marker();

  @GET
  void call() {}
}

void main() {
  final model = reflectClass(Model);
  for (final name in [#Model, #value, #call]) {
    assert(identical(
        model.declarations[name]!.metadata.single.reflectee, Model.d));
  }
  final constructor = model.declarations[#Model]! as MethodMirror;
  final method = model.declarations[#call]! as MethodMirror;
  assert(identical(
      constructor.parameters.single.metadata.single.reflectee, Model.d));
  assert(
      identical(method.parameters.single.metadata.single.reflectee, Model.d));
  assert(reflectClass(Api).declarations[#call]!.metadata.single.reflectee
      is Marker);
  assert(const Model(7).value == 7);
}
