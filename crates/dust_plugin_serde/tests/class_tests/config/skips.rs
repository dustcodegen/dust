use dust_ir::{AnnotationValueIr, ParamKind, SerdeFieldConfigIr, TypeIr};
use dust_plugin_api::{DustPlugin, SymbolPlan};
use dust_plugin_serde::register_plugin;

use crate::support::{class, constructor, constructor_param, field, library, span};

#[test]
fn supports_skipping_fields() {
    let plugin = register_plugin();
    let user_class = class(
        "User",
        vec![
            dust_ir::FieldIr {
                name: "password".to_owned(),
                ty: TypeIr::string(),
                span: span(10, 20),
                has_default: false,
                serde: Some(SerdeFieldConfigIr {
                    skip_serializing: true,
                    ..Default::default()
                }),
                configs: Vec::new(),
            },
            dust_ir::FieldIr {
                name: "token".to_owned(),
                ty: TypeIr::string(),
                span: span(10, 20),
                has_default: false,
                serde: Some(SerdeFieldConfigIr {
                    skip_deserializing: true,
                    default_value_source: Some("''".to_owned()),
                    ..Default::default()
                }),
                configs: Vec::new(),
            },
        ],
        vec![constructor(
            None,
            vec![
                constructor_param("password", TypeIr::string(), ParamKind::Named),
                constructor_param("token", TypeIr::string(), ParamKind::Named),
            ],
        )],
        &["dust_dart::Serialize", "dust_dart::Deserialize"],
    );

    let library = library(vec![user_class], vec![]);
    let contribution = plugin
        .generate(
            &library,
            &dust_plugin_api::PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .into_iter()
        .next()
        .expect("plugin must generate one contribution");

    assert_eq!(
        &contribution.top_level_functions[0],
        r#"Map<String, Object?> _$UserSerialize(User instance) {
  return <String, Object?>{
    'token': instance.token,
  };
}

Map<String, Object?> _$UserToJson(User instance) =>
    _$UserSerialize(instance);"#
    );
    assert_eq!(
        &contribution.top_level_functions[1],
        r#"// factory User.fromJson(Map<String, Object?> json) => _$UserFromJson(json);
User _$UserDeserialize(Map<String, Object?> json) {
  final passwordValue = JsonHelper.as<String>(
    json['password'],
    'password',
    'String',
  );
  final tokenValue = '';

  return User(password: passwordValue, token: tokenValue);
}

User _$UserFromJson(Map<String, Object?> json) =>
    _$UserDeserialize(json);"#
    );
}

#[test]
fn explicit_null_for_skipped_nullable_field_overrides_constructor_default() {
    let ty = TypeIr::string().nullable();
    let mut local = field("local", ty.clone());
    local.serde = Some(SerdeFieldConfigIr {
        skip_deserializing: true,
        default_value: Some(AnnotationValueIr::Null),
        default_value_source: Some("null".to_owned()),
        ..Default::default()
    });
    let mut param = constructor_param("local", ty, ParamKind::Named);
    param.is_required = false;
    param.has_default = true;
    param.default_value_source = Some("'constructor-default'".to_owned());
    let library = library(
        vec![class(
            "Local",
            vec![local],
            vec![constructor(None, vec![param])],
            &["dust_dart::Deserialize"],
        )],
        vec![],
    );
    let contribution = register_plugin()
        .generate(
            &library,
            &dust_plugin_api::PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        contribution.top_level_functions[0],
        r#"// factory Local.fromJson(Map<String, Object?> json) => _$LocalFromJson(json);
Local _$LocalDeserialize(Map<String, Object?> json) {
  final localValue = null;

  return Local(local: localValue);
}

Local _$LocalFromJson(Map<String, Object?> json) =>
    _$LocalDeserialize(json);"#
    );
}
