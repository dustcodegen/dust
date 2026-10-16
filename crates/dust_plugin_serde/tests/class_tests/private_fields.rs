use dust_ir::{ParamKind, TypeIr};
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan};
use dust_plugin_serde::register_plugin;

use super::support::{class, constructor, constructor_param, field, library};

#[test]
fn private_field_lookup_and_json_key_are_not_constructor_call_labels() {
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
        &["dust_dart::Deserialize"],
    );
    let contribution = register_plugin()
        .generate(
            &library(vec![model], vec![]),
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        contribution.top_level_functions,
        [
            r#"// factory User.fromJson(Map<String, Object?> json) => _$UserFromJson(json);
User _$UserDeserialize(Map<String, Object?> json) {
  final _nameValue = JsonHelper.as<String>(json['_name'], '_name', 'String');

  return User(name: _nameValue);
}

User _$UserFromJson(Map<String, Object?> json) =>
    _$UserDeserialize(json);"#
        ]
    );
}
