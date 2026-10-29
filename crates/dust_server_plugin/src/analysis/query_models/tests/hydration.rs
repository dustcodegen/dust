use super::*;

#[test]
fn hydrates_imported_errors_and_local_client_requirements() {
    let imported = param("filter", TypeIr::named("contracts.Filter"));
    let missing = param("missing", TypeIr::named("other.Filter"));
    let unknown = param("unknown", TypeIr::unknown());
    let handler_function = function();
    let mut server_plan = plan(
        &handler_function,
        vec![
            param_plan(&imported),
            param_plan(&missing),
            param_plan(&unknown),
        ],
    );
    let mut invalid = fact("package:contracts/models.dart");
    invalid.serializable = false;
    invalid.validatable = true;
    invalid.errors.push("unsupported field".to_owned());
    let mut analysis = WorkspaceAnalysisBuilder::default();
    analysis.add_string_set_value(
        QUERY_MODELS_ANALYSIS_KEY,
        serde_json::to_string(&invalid).unwrap(),
    );

    let diagnostics = hydrate_query_models(&file(), &mut server_plan, &analysis.build(), true);
    assert_eq!(diagnostics.len(), 3);
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("unsupported field"))
    );
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("derive Serialize"))
    );
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("prefix `other`"))
    );
    assert!(server_plan.functions[0].params[0].validated);
    assert!(!server_plan.functions[0].params[0].serde_deserialized);

    let local = param("filter", TypeIr::named("Filter"));
    let handler_function = function();
    let mut server_plan = plan(&handler_function, vec![param_plan(&local)]);
    let mut file = file();
    file.classes.push(ClassIr {
        kind: ClassKindIr::Class,
        name: "Filter".to_owned(),
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
        traits: vec![TraitApplicationIr {
            symbol: SymbolId::new("dust_dart::Deserialize"),
            span: span(),
        }],
        configs: Vec::new(),
        serde: None,
    });
    assert!(
        hydrate_query_models(
            &file,
            &mut server_plan,
            &WorkspaceAnalysisBuilder::default().build(),
            true,
        )[0]
        .message
        .contains("derive Serialize")
    );
}

#[test]
fn round_trips_every_query_conversion_fact() {
    let plans = [
        QueryConversionPlan {
            key: "value".to_owned(),
            aliases: Vec::new(),
            kind: QueryConversionKind::Scalar(TypeIr::bool().nullable()),
        },
        QueryConversionPlan {
            key: "names".to_owned(),
            aliases: Vec::new(),
            kind: QueryConversionKind::StringList,
        },
        QueryConversionPlan {
            key: "ids".to_owned(),
            aliases: vec!["id".to_owned()],
            kind: QueryConversionKind::ScalarList(TypeIr::int()),
        },
    ];
    let facts = plans
        .into_iter()
        .map(QueryConversionFact::from)
        .collect::<Vec<_>>();
    let model = QueryModelFact {
        package: "contracts".to_owned(),
        library: "package:contracts/models.dart".to_owned(),
        name: "Filter".to_owned(),
        serializable: true,
        validatable: false,
        errors: Vec::new(),
        conversions: facts,
    };
    let hydrated = model.to_plan();
    assert_eq!(hydrated.conversions.len(), 3);
    assert_eq!(hydrated.conversions[2].aliases, ["id"]);
    assert!(matches!(
        hydrated.conversions[1].kind,
        QueryConversionKind::StringList
    ));
    assert!(matches!(
        hydrated.conversions[2].kind,
        QueryConversionKind::ScalarList(_)
    ));

    for name in ["double", "num", "unknown"] {
        let fact = QueryConversionFact {
            key: name.to_owned(),
            aliases: Vec::new(),
            kind: QueryConversionFactKind::Scalar(name.to_owned()),
        };
        assert!(matches!(
            QueryConversionPlan::from(&fact).kind,
            QueryConversionKind::Scalar(_)
        ));
    }
}
