use super::*;

#[test]
fn rejects_models_without_dust_derive_contracts() {
    let body = param("input", TypeIr::named("contracts.CreateTodo"));
    let handler = function(TypeIr::named("Future"));
    let mut unsupported = fact("CreateTodo");
    unsupported.deserializable = false;
    unsupported.serializable = false;
    let mut server = plan(&handler, Some(&body));

    let diagnostics = hydrate_payload_models(&file(), &mut server, &models(&[unsupported]), true);
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(Diagnostic::has_labels));
    assert!(!server.functions[0].params[0].serde_deserialized);
}

#[test]
fn reports_malformed_facts_in_body_hydration() {
    let mut builder = WorkspaceAnalysisBuilder::default();
    builder.add_string_set_value(PAYLOAD_MODELS_KEY, "{");

    assert!(PayloadCapabilityIndex::from_analysis(&builder.build()).is_err());
}

#[test]
fn reports_import_resolution_failures_at_each_payload_boundary() {
    let facts = vec![fact("Todo")];
    let models = models(&facts);
    assert!(imported_fact(&file(), &models, "Todo").is_err());

    let mut missing = file();
    missing.import_directives.clear();
    assert!(imported_fact(&missing, &models, "contracts.Todo").is_err());

    let mut relative = file();
    relative.import_directives[0].uri = "../models.dart".to_owned();
    assert!(imported_fact(&relative, &models, "contracts.Todo").is_err());

    let body = param("input", TypeIr::named("contracts.Todo"));
    let handler = function(TypeIr::named("Future"));
    let mut server = plan(&handler, Some(&body));
    assert_eq!(
        hydrate_payload_models(&missing, &mut server, &models, true).len(),
        1
    );
    assert!(
        response_serde_types(&missing, &TypeIr::named("contracts.Todo"), span(), &models).is_err()
    );
}

#[test]
fn resolves_reexports_and_rejects_ambiguous_package_models() {
    let mut reexported = fact("Todo");
    reexported.library = "package:contracts/src/todo.dart".to_owned();
    assert_eq!(
        imported_fact(&file(), &models(&[reexported]), "contracts.Todo")
            .unwrap()
            .name
            .as_str(),
        "Todo"
    );

    let mut second = fact("Todo");
    second.library = "package:contracts/src/second.dart".to_owned();
    let mut third = fact("Todo");
    third.library = "package:contracts/src/third.dart".to_owned();
    assert!(imported_fact(&file(), &models(&[second, third]), "contracts.Todo").is_err());
}

#[test]
fn rejects_unknown_models_and_ignores_non_named_response_shapes() {
    assert!(
        response_serde_types(
            &file(),
            &TypeIr::named("contracts.Unknown"),
            span(),
            &models(&[])
        )
        .is_err()
    );
    assert!(
        response_serde_types(&file(), &TypeIr::Dynamic, span(), &models(&[]))
            .unwrap()
            .is_empty()
    );
    for native in [
        TypeIr::named("Uint8List"),
        TypeIr::generic("Stream", vec![TypeIr::generic("List", vec![TypeIr::int()])]),
    ] {
        assert!(
            response_serde_types(&file(), &native, span(), &models(&[]))
                .unwrap()
                .is_empty()
        );
    }
}
