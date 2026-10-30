use dust_ir::{ClassIr, ClassKindIr, SymbolId, TraitApplicationIr};
use dust_plugin_api::WorkspaceAnalysisBuilder;

use super::*;

#[test]
fn collects_known_models_and_their_capabilities() {
    let mut source = DartFileIr::empty(
        "/contracts",
        "contracts",
        "/contracts/lib/models.dart",
        "/contracts/lib/models.g.dart",
        span(),
    );
    source.classes.push(ClassIr {
        kind: ClassKindIr::Class,
        name: "Todo".to_owned(),
        type_parameters: Vec::new(),
        mixins: Vec::new(),
        is_abstract: false,
        is_interface: false,
        superclass_name: None,
        superclass: None,
        span: span(),
        fields: Vec::new(),
        constructors: Vec::new(),
        methods: Vec::new(),
        traits: ["Deserialize", "Serialize", "Validate"]
            .into_iter()
            .map(|name| TraitApplicationIr {
                symbol: SymbolId::new(format!("dust_dart::{name}")),
                span: span(),
            })
            .collect(),
        configs: Vec::new(),
        serde: None,
    });
    let mut plain = source.classes[0].clone();
    plain.name = "Plain".to_owned();
    plain.constructors.clear();
    plain.methods.clear();
    plain.traits.clear();
    source.classes.push(plain);

    let mut builder = WorkspaceAnalysisBuilder::default();
    collect_payload_model_workspace_analysis_ir(&source, &mut builder);
    let values = model_facts(&builder.build()).unwrap();
    let todo = values.iter().find(|value| value.name == "Todo").unwrap();
    assert!(todo.deserializable && todo.serializable && todo.validatable);
    let plain = values.iter().find(|value| value.name == "Plain").unwrap();
    assert!(!plain.deserializable && !plain.serializable && !plain.validatable);

    let outside = DartFileIr::empty("/pkg", "pkg", "outside.dart", "out.g.dart", span());
    let mut builder = WorkspaceAnalysisBuilder::default();
    collect_payload_model_workspace_analysis_ir(&outside, &mut builder);
    assert!(builder.build().string_set(PAYLOAD_MODELS_KEY).is_none());
}
