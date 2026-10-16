use dust_ir::{FieldIr, NameIr, TypeIr, TypeParamIr};
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan};
use dust_plugin_derive::register_plugin;

use crate::support::{class_with_traits, members_for_class, sample_library, span};

#[test]
fn generic_copy_helpers_bind_class_parameters_and_keep_null_distinct_from_omission() {
    let mut library = sample_library(&["dust_dart::CopyWith"]);
    let class = &mut library.classes[0];
    class.name = "Box".to_owned();
    class.type_parameters = vec![TypeParamIr {
        name: NameIr {
            source: "T".to_owned(),
            short: "T".to_owned(),
            prefix: None,
            span: span(0, 1),
        },
        bound: None,
        span: span(0, 1),
    }];
    class.fields = vec![FieldIr {
        name: "value".to_owned(),
        ty: TypeIr::named("T"),
        span: span(1, 2),
        has_default: false,
        serde: None,
        configs: Vec::new(),
    }];
    class.constructors[0].name = Some("named".to_owned());
    class.constructors[0].params.truncate(1);
    class.constructors[0].params[0].name = "value".to_owned();
    class.constructors[0].params[0].ty = TypeIr::named("T");
    let contribution = register_plugin()
        .generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        contribution.shared_helpers,
        [
            "bool _copyWithAcceptsNull<T>() => null is T;",
            "final class _BoxCopyWithUnset {\n  const _BoxCopyWithUnset();\n}\n\nconst _boxCopyWithUnset = _BoxCopyWithUnset();",
        ]
    );
    assert_eq!(
        members_for_class(&contribution, "Box"),
        [
            r#"/// Creates a copy of this `Box` with selected fields replaced.
/// Null clears a field only when its instantiated type accepts null.
///
/// Usage:
/// ```dart
/// final updated = box.copyWith();
/// ```
@pragma('vm:prefer-inline')
_$BoxCopyWith<Box<T>, T> get copyWith => _$BoxCopyWithImpl<Box<T>, T>(this as Box<T>, (value2) => value2);"#
        ]
    );
    assert_eq!(
        contribution.support_types,
        [r#"// CopyWith API inspired by Freezed.

/// @nodoc
abstract class _$BoxCopyWith<$Res, T> {
  $Res call({
    T? value,
  });
}

/// @nodoc
final class _$BoxCopyWithImpl<$Res, T> implements _$BoxCopyWith<$Res, T> {
  const _$BoxCopyWithImpl(this._self, this._then);

  final Box<T> _self;
  final $Res Function(Box<T>) _then;

  @override
  @pragma('vm:prefer-inline')
  $Res call({
    Object? value = _boxCopyWithUnset,
  }) {
    return _then(
      Box<T>.named(
        identical(value, _boxCopyWithUnset) || (value == null && !_copyWithAcceptsNull<T>())
            ? _self.value
            : value as T,
      )
    );
  }
}"#]
    );
}

#[test]
fn raw_generic_fields_do_not_invent_missing_nested_helper_arguments() {
    let mut library = sample_library(&["dust_dart::CopyWith"]);
    let parent = &mut library.classes[0];
    parent.name = "Parent".to_owned();
    parent.fields.truncate(1);
    parent.fields[0].name = "child".to_owned();
    parent.fields[0].ty = TypeIr::named("Child");
    parent.constructors[0].params.truncate(1);
    parent.constructors[0].params[0].name = "child".to_owned();
    parent.constructors[0].params[0].ty = TypeIr::named("Child");
    let mut child = class_with_traits("Child", &["dust_dart::CopyWith"]);
    child.type_parameters = vec![TypeParamIr {
        name: NameIr {
            source: "T".to_owned(),
            short: "T".to_owned(),
            prefix: None,
            span: span(0, 1),
        },
        bound: None,
        span: span(0, 1),
    }];
    child.fields.truncate(1);
    child.fields[0].name = "value".to_owned();
    child.fields[0].ty = TypeIr::named("T");
    child.constructors[0].params.truncate(1);
    child.constructors[0].params[0].name = "value".to_owned();
    child.constructors[0].params[0].ty = TypeIr::named("T");
    library.classes.push(child);
    let contribution = register_plugin()
        .generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        contribution.support_types[0],
        r#"// CopyWith API inspired by Freezed.

/// @nodoc
abstract class _$ParentCopyWith<$Res> {
  $Res call({
    Child? child,
  });
}

/// @nodoc
final class _$ParentCopyWithImpl<$Res> implements _$ParentCopyWith<$Res> {
  const _$ParentCopyWithImpl(this._self, this._then);

  final Parent _self;
  final $Res Function(Parent) _then;

  @override
  @pragma('vm:prefer-inline')
  $Res call({
    Object? child = null,
  }) {
    return _then(
      Parent(
        child == null ? _self.child : child as Child,
      )
    );
  }
}"#
    );
}
