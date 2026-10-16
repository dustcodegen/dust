use dust_ir::{ParamKind, TypeIr};
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan};
use dust_plugin_derive::register_plugin;

use super::support::{class, constructor, constructor_param, field, library, members_for_class};

#[test]
fn private_fields_keep_private_access_and_public_copy_labels() {
    let model = class(
        "User",
        vec![field("_name", TypeIr::string())],
        vec![constructor(
            None,
            vec![constructor_param(
                "_name",
                TypeIr::string(),
                ParamKind::Named,
            )],
        )],
        &["dust_dart::CopyWith"],
    );
    let library = library(vec![model]);
    let plugin = register_plugin();
    assert_eq!(plugin.validate(&library), []);
    let contribution = plugin
        .generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        members_for_class(&contribution, "User"),
        [
            r#"/// Creates a copy of this `User` with selected fields replaced.
///
/// Usage:
/// ```dart
/// final updated = user.copyWith(name: 'John');
/// ```
@pragma('vm:prefer-inline')
_$UserCopyWith<User> get copyWith => _$UserCopyWithImpl<User>(this as User, (value) => value);"#
        ]
    );
    assert_eq!(
        contribution.support_types,
        [r#"// CopyWith API inspired by Freezed.

/// @nodoc
abstract class _$UserCopyWith<$Res> {
  $Res call({
    String? name,
  });
}

/// @nodoc
final class _$UserCopyWithImpl<$Res> implements _$UserCopyWith<$Res> {
  const _$UserCopyWithImpl(this._self, this._then);

  final User _self;
  final $Res Function(User) _then;

  @override
  @pragma('vm:prefer-inline')
  $Res call({
    Object? name = null,
  }) {
    return _then(
      User(
        name: name == null ? _self._name : name as String,
      )
    );
  }
}"#]
    );
}

#[test]
fn conflicting_public_copy_labels_are_rejected_instead_of_emitting_duplicates() {
    let fields = vec![
        field("_name", TypeIr::string()),
        field("name", TypeIr::string()),
    ];
    let constructor = constructor(
        None,
        fields
            .iter()
            .map(|field| constructor_param(&field.name, field.ty.clone(), ParamKind::Positional))
            .collect(),
    );
    let model = class(
        "Collision",
        fields,
        vec![constructor],
        &["dust_dart::CopyWith"],
    );
    let diagnostics = register_plugin().validate(&library(vec![model]));
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        [
            "`CopyWith` fields `_name` and `name` on class `Collision` share public argument name `name`",
        ]
    );
}
