use dust_ir::{MethodIr, NameIr, TypeIr, TypeParamIr};
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan};
use dust_plugin_derive::register_plugin;

use crate::support::{class_with_traits, sample_library, span};

#[test]
fn aliases_core_names_without_renaming_source_binders() {
    let mut library = sample_library(&[
        "dust_dart::CopyWith",
        "dust_dart::ToString",
        "dust_dart::Eq",
    ]);
    library.classes[0].type_parameters = ["String", "Object", "bool", "int", "identical", "pragma"]
        .into_iter()
        .map(|name| TypeParamIr {
            name: NameIr {
                source: name.into(),
                short: name.into(),
                prefix: None,
                span: span(0, 1),
            },
            bound: None,
            span: span(0, 1),
        })
        .collect();
    let contribution = register_plugin()
        .generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        contribution
            .shared_helpers
            .iter()
            .take(6)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        &[
            "typedef _DustCoreObject = Object;",
            "typedef _DustCoreString = String;",
            "typedef _DustCoreBool = bool;",
            "typedef _DustCoreInt = int;",
            "const bool Function(Object?, Object?) _dustCoreIdentical = identical;",
            "typedef _DustCorePragma = pragma;",
        ]
    );
}

fn parameter(name: &str) -> TypeParamIr {
    TypeParamIr {
        name: NameIr {
            source: name.into(),
            short: name.into(),
            prefix: None,
            span: span(0, 1),
        },
        bound: None,
        span: span(0, 1),
    }
}

fn method(name: &str) -> MethodIr {
    MethodIr {
        name: name.into(),
        is_static: false,
        is_external: false,
        return_type: TypeIr::bool(),
        has_body: true,
        body_source: Some("=> true".into()),
        params: Vec::new(),
        span: span(1, 2),
        traits: Vec::new(),
        configs: Vec::new(),
    }
}

#[test]
fn alias_names_avoid_library_declarations_fields_and_methods() {
    let mut library = sample_library(&["dust_dart::CopyWith"]);
    let class = &mut library.classes[0];
    class.type_parameters = vec![parameter("Object")];
    class.fields[0].name = "_DustCoreObject2".into();
    class.constructors[0].params[0].name = "_DustCoreObject2".into();
    class.fields[1].name = "identical".into();
    class.constructors[0].params[1].name = "identical".into();
    class.methods = vec![
        method("_DustCoreObject3"),
        method("_dustCoreIdentical2"),
        method("pragma"),
    ];
    library.classes.extend([
        class_with_traits("_DustCoreObject", &[]),
        class_with_traits("_dustCoreIdentical", &[]),
    ]);
    let contribution = register_plugin()
        .generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        contribution
            .shared_helpers
            .iter()
            .take(3)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "typedef _DustCoreObject4 = Object;",
            "const bool Function(Object?, Object?) _dustCoreIdentical3 = identical;",
            "typedef _DustCorePragma = pragma;",
        ]
    );
}

#[test]
fn unknown_field_fallback_uses_core_object_not_the_source_binder() {
    let mut library = sample_library(&["dust_dart::CopyWith"]);
    let class = &mut library.classes[0];
    class.type_parameters = vec![parameter("Object")];
    class.fields.truncate(1);
    class.fields[0].ty = TypeIr::unknown();
    class.constructors[0].params.truncate(1);
    class.constructors[0].params[0].ty = TypeIr::unknown();
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
abstract class _$UserCopyWith<$Res, Object> {
  $Res call({
    _DustCoreObject? id,
  });
}

/// @nodoc
final class _$UserCopyWithImpl<$Res, Object> implements _$UserCopyWith<$Res, Object> {
  const _$UserCopyWithImpl(this._self, this._then);

  final User<Object> _self;
  final $Res Function(User<Object>) _then;

  @override
  @pragma('vm:prefer-inline')
  $Res call({
    _DustCoreObject? id = _userCopyWithUnset,
  }) {
    return _then(
      User<Object>(
        identical(id, _userCopyWithUnset) || (id == null && !_copyWithAcceptsNull<_DustCoreObject?>())
            ? _self.id
            : id as _DustCoreObject?,
      )
    );
  }
}"#
    );
}
