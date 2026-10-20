use super::*;
use dust_ir::{ClassIr, NameIr, SerdeFieldConfigIr, TypeParamIr};
use dust_plugin_api::{SymbolPlan, WorkspaceAnalysisBuilder};

fn parameter(name: &str) -> TypeParamIr {
    TypeParamIr {
        name: NameIr {
            source: name.into(),
            short: name.into(),
            prefix: None,
            span: span(1, 2),
        },
        bound: Some(TypeIr::num()),
        span: span(1, 2),
    }
}

fn target(name: &str, ty: TypeIr) -> ClassIr {
    let mut target = class(
        "Captured",
        vec![field("value", ty.clone())],
        vec![constructor(
            None,
            vec![constructor_param("value", ty, ParamKind::Positional)],
        )],
        &["dust_dart::Serialize", "dust_dart::Deserialize"],
    );
    target.type_parameters.push(parameter(name));
    target
}

#[test]
fn class_parameters_are_neither_sdk_scalars_nor_unknown_external_models() {
    for name in [
        "T", "DateTime", "Uri", "BigInt", "Object", "List", "Set", "Map", "String",
    ] {
        for nullable in [false, true] {
            let ty = if nullable {
                TypeIr::named(name).nullable()
            } else {
                TypeIr::named(name)
            };
            let result = register_plugin().validate(&library(vec![target(name, ty)], vec![]));
            assert_eq!(result.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
                       ["Serialize", "Deserialize"].map(|direction| format!(
                           "`{direction}` cannot automatically map class parameter `{name}` on `Captured.value`; use `SerDe(using: ...)`")));
            for diagnostic in result {
                assert!(diagnostic.is_error());
                assert_eq!(diagnostic.labels.len(), 1);
                assert_eq!(diagnostic.labels[0].range, span(10, 20).range);
            }
        }
    }
}

#[test]
fn class_parameters_are_checked_inside_real_containers() {
    for ty in [
        TypeIr::list_of(TypeIr::named("T")),
        TypeIr::generic("Set", vec![TypeIr::named("T")]),
        TypeIr::map_of(
            TypeIr::string(),
            TypeIr::list_of(TypeIr::named("T").nullable()),
        ),
    ] {
        let result = register_plugin().validate(&library(vec![target("T", ty)], vec![]));
        assert_eq!(
            result
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>(),
            [
                "`Serialize` cannot automatically map class parameter `T` on `Captured.value`; use `SerDe(using: ...)`",
                "`Deserialize` cannot automatically map class parameter `T` on `Captured.value`; use `SerDe(using: ...)`",
            ]
        );
    }
}

#[test]
fn phantom_parameters_concrete_fields_and_explicit_codec_choices_are_not_blanket_rejected() {
    let mut codec = target("DateTime", TypeIr::named("DateTime"));
    codec.fields[0].serde = Some(SerdeFieldConfigIr {
        codec_source: Some("selectedCodec".into()),
        ..Default::default()
    });
    let concrete = target("T", TypeIr::named("DateTime"));
    let qualified = target("DateTime", TypeIr::named("external.DateTime"));
    let builtin = target("String", TypeIr::string());
    for target in [codec, concrete, qualified, builtin] {
        assert_eq!(
            register_plugin().validate(&library(vec![target], vec![])),
            []
        );
    }
}

#[test]
fn class_parameters_shadow_local_and_workspace_json_capable_names() {
    let plugin = register_plugin();
    let declared = class(
        "T",
        vec![],
        vec![constructor(None, vec![])],
        &["dust_dart::Serialize", "dust_dart::Deserialize"],
    );
    let mut analysis = WorkspaceAnalysisBuilder::default();
    plugin.collect_workspace_analysis_ir(&library(vec![declared.clone()], vec![]), &mut analysis);
    let mut plan = SymbolPlan::default();
    plan.set_workspace_analysis(std::sync::Arc::new(analysis.build()));
    for local in [false, true] {
        let mut classes = vec![target("T", TypeIr::named("T"))];
        if local {
            classes.push(declared.clone());
        }
        let mut file = library(classes, vec![]);
        let result = plugin.validate_with_plan(&file, &plan);
        assert_eq!(
            result
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>(),
            [
                "`Serialize` cannot automatically map class parameter `T` on `Captured.value`; use `SerDe(using: ...)`",
                "`Deserialize` cannot automatically map class parameter `T` on `Captured.value`; use `SerDe(using: ...)`",
            ]
        );
        file.classes[0].type_parameters.clear();
        assert_eq!(plugin.validate_with_plan(&file, &plan), []);
    }
}

#[test]
fn skipped_parameter_directions_do_not_require_automatic_mapping() {
    for (serialize, deserialize) in [(false, true), (true, false), (true, true)] {
        let mut target = target("T", TypeIr::named("T").nullable());
        target.fields[0].serde = Some(SerdeFieldConfigIr {
            skip_serializing: serialize,
            skip_deserializing: deserialize,
            default_value: Some(AnnotationValueIr::Null),
            default_value_source: Some("null".into()),
            ..Default::default()
        });
        let result = register_plugin().validate(&library(vec![target], vec![]));
        let expected = if serialize && deserialize {
            vec![]
        } else {
            let direction = if serialize {
                "Deserialize"
            } else {
                "Serialize"
            };
            vec![format!(
                "`{direction}` cannot automatically map class parameter `T` on `Captured.value`; use `SerDe(using: ...)`"
            )]
        };
        assert_eq!(
            result.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
            expected
        );
    }
}
