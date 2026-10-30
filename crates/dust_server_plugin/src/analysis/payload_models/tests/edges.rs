use super::*;

#[test]
fn accepts_legacy_body_codec_methods_for_generated_clients() {
    let body = param("input", TypeIr::named("contracts.CreateTodo"));
    let handler = function(TypeIr::named("Future"));
    let mut compatible = fact("CreateTodo");
    compatible.deserializable = false;
    compatible.deserialize_method = true;
    compatible.serializable = false;
    compatible.to_json = true;
    let mut server = plan(&handler, Some(&body));

    assert!(
        hydrate_payload_models(&file(), &mut server, &analysis(&[compatible]), true).is_empty()
    );
    assert!(!server.functions[0].params[0].serde_deserialized);
}

#[test]
fn reports_malformed_facts_in_body_hydration() {
    let body = param("input", TypeIr::named("contracts.CreateTodo"));
    let handler = function(TypeIr::named("Future"));
    let mut server = plan(&handler, Some(&body));
    let mut builder = WorkspaceAnalysisBuilder::default();
    builder.add_string_set_value(PAYLOAD_MODELS_KEY, "{");

    let errors = hydrate_payload_models(&file(), &mut server, &builder.build(), true);

    assert_eq!(errors.len(), 1);
    assert!(
        errors[0]
            .message
            .contains("invalid dust_server.payload_models.v1")
    );
}

#[test]
fn reports_import_resolution_failures_at_each_payload_boundary() {
    let facts = vec![fact("Todo")];
    assert!(imported_fact(&file(), &facts, "Todo").is_err());

    let mut missing = file();
    missing.import_directives.clear();
    assert!(imported_fact(&missing, &facts, "contracts.Todo").is_err());

    let mut relative = file();
    relative.import_directives[0].uri = "../models.dart".to_owned();
    assert!(imported_fact(&relative, &facts, "contracts.Todo").is_err());

    let body = param("input", TypeIr::named("contracts.Todo"));
    let handler = function(TypeIr::named("Future"));
    let mut server = plan(&handler, Some(&body));
    assert_eq!(
        hydrate_payload_models(&missing, &mut server, &analysis(&facts), true).len(),
        1
    );
    assert!(
        response_serde_types(
            &missing,
            &TypeIr::named("contracts.Todo"),
            &analysis(&facts)
        )
        .is_err()
    );
}

#[test]
fn resolves_reexports_and_rejects_ambiguous_package_models() {
    let mut reexported = fact("Todo");
    reexported.library = "package:contracts/src/todo.dart".to_owned();
    assert_eq!(
        imported_fact(&file(), &[reexported], "contracts.Todo")
            .unwrap()
            .map(|value| value.name.as_str()),
        Some("Todo")
    );

    let mut second = fact("Todo");
    second.library = "package:contracts/src/second.dart".to_owned();
    let mut third = fact("Todo");
    third.library = "package:contracts/src/third.dart".to_owned();
    assert!(imported_fact(&file(), &[second, third], "contracts.Todo").is_err());
}

#[test]
fn ignores_unknown_and_non_named_response_shapes() {
    assert!(
        response_serde_types(&file(), &TypeIr::named("contracts.Unknown"), &analysis(&[]))
            .unwrap()
            .is_empty()
    );
    assert!(
        response_serde_types(&file(), &TypeIr::Dynamic, &analysis(&[]))
            .unwrap()
            .is_empty()
    );
}
