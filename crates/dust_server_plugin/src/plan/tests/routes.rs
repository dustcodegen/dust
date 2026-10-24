use super::*;

#[test]
fn plans_named_top_level_routes() {
    let mut library = file();
    let mut unrelated_library_annotation = annotation("other::Config", "/ignored", []);
    unrelated_library_annotation.resolved_symbol = None;
    library.library_annotations = vec![
        unrelated_library_annotation,
        annotation(
            "dust_server::Routes",
            "/api",
            [
                ("name", AnnotationValueIr::String("apiRoutes".to_owned())),
                (
                    "tags",
                    AnnotationValueIr::List(vec![AnnotationValueIr::String("api".to_owned())]),
                ),
            ],
        ),
    ];
    let mut unresolved_function_annotation = annotation("GET", "/ignored", []);
    unresolved_function_annotation.resolved_symbol = None;
    library.functions = vec![FunctionIr {
        name: NameIr {
            source: "health".to_owned(),
            short: "health".to_owned(),
            prefix: None,
            span: span(),
        },
        return_type: TypeIr::named("Health"),
        params: Vec::new(),
        annotations: vec![
            unresolved_function_annotation,
            annotation(
                "dust_server::GET",
                "/health",
                [
                    ("summary", AnnotationValueIr::String("Health".to_owned())),
                    (
                        "description",
                        AnnotationValueIr::String("Checks readiness".to_owned()),
                    ),
                    (
                        "tags",
                        AnnotationValueIr::List(vec![AnnotationValueIr::String(
                            "probe".to_owned(),
                        )]),
                    ),
                    (
                        "operationId",
                        AnnotationValueIr::String("healthCheck".to_owned()),
                    ),
                    ("deprecated", AnnotationValueIr::Bool(true)),
                    ("hidden", AnnotationValueIr::Bool(true)),
                ],
            ),
        ],
        span: span(),
    }];

    let plan = plan_server(&library).unwrap().unwrap();
    assert_eq!(plan.function_name, "apiRoutes");
    assert_eq!(plan.function_prefix, "/api");
    assert_eq!(plan.functions.len(), 1);
    assert_eq!(plan.functions[0].callable.name(), "health");
    assert_eq!(
        plan.functions[0].metadata,
        RouteMetadataPlan {
            operation_id: "healthCheck".to_owned(),
            summary: Some("Health".to_owned()),
            description: Some("Checks readiness".to_owned()),
            tags: vec!["api".to_owned(), "probe".to_owned()],
            deprecated: true,
            hidden: true,
        }
    );

    library.library_annotations[1].named_args.insert(
        "name".to_owned(),
        AnnotationValueIr::String("switch".to_owned()),
    );
    assert!(
        messages(&library)
            .iter()
            .any(|value| value.contains("not a valid Dart identifier"))
    );
}

#[test]
fn ignores_plain_controller_methods_without_server_metadata() {
    let plain = method("helper", Vec::new());
    let handler = method("read", vec![config("dust_server::GET", Some("/read"))]);
    let mut library = file();
    library.classes = vec![class(vec![plain.clone(), handler.clone()])];

    let plan = plan_server(&library).unwrap().unwrap();
    assert_eq!(plan.controllers[0].handlers.len(), 1);
    assert_eq!(plan.controllers[0].handlers[0].callable.name(), "read");

    library.classes[0].configs.clear();
    library.classes[0].methods = vec![plain];
    library.functions = vec![FunctionIr {
        name: NameIr {
            source: "helper".to_owned(),
            short: "helper".to_owned(),
            prefix: None,
            span: span(),
        },
        return_type: TypeIr::named("void"),
        params: Vec::new(),
        annotations: Vec::new(),
        span: span(),
    }];
    assert!(plan_server(&library).unwrap().is_none());

    library.classes[0].methods = vec![handler];
    library.functions.clear();
    assert!(
        messages(&library)
            .iter()
            .any(|value| value.contains("do not declare a supported handler"))
    );
}
