use dust_ir::{AnnotationValueIr, SerdeClassConfigIr, SerdeFieldConfigIr};
use dust_ir::{NameIr, ParamKind, TypeIr, TypeParamIr};
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan};
use dust_plugin_serde::register_plugin;

use super::support::{
    class, constructor, constructor_param, field, library, members_for_class, span,
};

#[test]
fn generic_helpers_forward_parameters_and_construct_typed_results() {
    let mut model = class(
        "Phantom",
        vec![field("label", TypeIr::string())],
        vec![constructor(
            None,
            vec![constructor_param(
                "label",
                TypeIr::string(),
                ParamKind::Positional,
            )],
        )],
        &["dust_dart::Serialize", "dust_dart::Deserialize"],
    );
    model.type_parameters.push(TypeParamIr {
        name: NameIr {
            source: "T".into(),
            short: "T".into(),
            prefix: None,
            span: span(1, 2),
        },
        bound: Some(TypeIr::num()),
        span: span(1, 2),
    });
    let result = register_plugin()
        .generate(
            &library(vec![model], vec![]),
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        members_for_class(&result, "Phantom"),
        [
            "Map<String, Object?> serialize() => _$PhantomSerialize<T>(this as Phantom<T>);",
            "Map<String, Object?> toJson() => serialize();",
        ]
    );
    assert_eq!(
        result.support_types,
        [
            r#"/// Typed JSON Serializer for `Phantom<T>`.
final class $PhantomSerializer<T extends num> implements Serializer<Phantom<T>, Map<String, Object?>> {
  /// Creates a typed JSON codec.
  const $PhantomSerializer();

  @override
  Map<String, Object?> serialize(Phantom<T> value) => _$PhantomSerialize<T>(value);
}"#,
            r#"/// Typed JSON Deserializer for `Phantom<T>`.
final class $PhantomDeserializer<T extends num> implements Deserializer<Phantom<T>, Map<String, Object?>> {
  /// Creates a typed JSON codec.
  const $PhantomDeserializer();

  @override
  Phantom<T> deserialize(Map<String, Object?> json) => _$PhantomDeserialize<T>(json);
}"#,
        ]
    );
    assert_eq!(
        result.top_level_functions,
        [
            r#"Map<String, Object?> _$PhantomSerialize<T extends num>(Phantom<T> instance) {
  return <String, Object?>{
    'label': instance.label,
  };
}

Map<String, Object?> _$PhantomToJson<T extends num>(Phantom<T> instance) =>
    _$PhantomSerialize<T>(instance);"#,
            r#"// factory Phantom.fromJson(Map<String, Object?> json) => _$PhantomFromJson(json);
Phantom<T> _$PhantomDeserialize<T extends num>(Map<String, Object?> json) {
  final labelValue = JsonHelper.as<String>(json['label'], 'label', 'String');

  return Phantom<T>(labelValue);
}

Phantom<T> _$PhantomFromJson<T extends num>(Map<String, Object?> json) =>
    _$PhantomDeserialize<T>(json);"#,
        ]
    );
}

#[test]
fn generic_boolean_defaults_use_typed_short_circuit_logic_in_both_alias_shapes() {
    for (value, alias) in [(true, false), (false, true)] {
        let mut flag = field("flag", TypeIr::bool());
        flag.serde = Some(SerdeFieldConfigIr {
            default_value: Some(AnnotationValueIr::Bool(value)),
            default_value_source: Some(value.to_string()),
            aliases: if alias { vec!["old".into()] } else { vec![] },
            ..Default::default()
        });
        let mut model = class(
            "Flag",
            vec![flag],
            vec![constructor(
                None,
                vec![constructor_param(
                    "flag",
                    TypeIr::bool(),
                    ParamKind::Positional,
                )],
            )],
            &["dust_dart::Deserialize"],
        );
        model.type_parameters.push(TypeParamIr {
            name: NameIr {
                source: "T".into(),
                short: "T".into(),
                prefix: None,
                span: span(1, 2),
            },
            bound: None,
            span: span(1, 2),
        });
        model.serde = Some(SerdeClassConfigIr::default());
        let result = register_plugin()
            .generate(
                &library(vec![model], vec![]),
                &PluginContext {
                    symbol_plan: &SymbolPlan::default(),
                },
            )
            .remove(0);
        let body = if alias {
            r#"  var rawFlagKey = 'flag';
  Object? rawFlag;
  if (json.containsKey('flag')) {
    rawFlag = json['flag'];
  } else if (json.containsKey('old')) {
    rawFlagKey = 'old';
    rawFlag = json['old'];
  }
  final flagValue = (json.containsKey('flag') || json.containsKey('old')) && JsonHelper.as<bool>(rawFlag, rawFlagKey, 'bool');"#
        } else {
            "  final flagValue = !json.containsKey('flag') || JsonHelper.as<bool>(json['flag'], 'flag', 'bool');"
        };
        assert_eq!(
            result.top_level_functions,
            [format!(
                r#"// factory Flag.fromJson(Map<String, Object?> json) => _$FlagFromJson(json);
Flag<T> _$FlagDeserialize<T>(Map<String, Object?> json) {{
{body}

  return Flag<T>(flagValue);
}}

Flag<T> _$FlagFromJson<T>(Map<String, Object?> json) =>
    _$FlagDeserialize<T>(json);"#
            )]
        );
    }
}

#[test]
fn serialize_only_does_not_plan_unused_decoder_aliases() {
    let mut model = class(
        "Only",
        vec![field("amount", TypeIr::int())],
        vec![constructor(
            None,
            vec![constructor_param(
                "amount",
                TypeIr::int(),
                ParamKind::Positional,
            )],
        )],
        &["dust_dart::Serialize"],
    );
    model.type_parameters.push(TypeParamIr {
        name: NameIr {
            source: "JsonHelper".into(),
            short: "JsonHelper".into(),
            prefix: None,
            span: span(1, 2),
        },
        bound: None,
        span: span(1, 2),
    });
    let result = register_plugin()
        .generate(
            &library(vec![model], vec![]),
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(result.shared_helpers, Vec::<String>::new());
    assert_eq!(
        result.top_level_functions,
        [
            r#"Map<String, Object?> _$OnlySerialize<JsonHelper>(Only<JsonHelper> instance) {
  return <String, Object?>{
    'amount': instance.amount,
  };
}

Map<String, Object?> _$OnlyToJson<JsonHelper>(Only<JsonHelper> instance) =>
    _$OnlySerialize<JsonHelper>(instance);"#
        ]
    );
}
