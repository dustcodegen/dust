use std::collections::BTreeMap;

use dust_ir::{
    AnnotationIr, AnnotationValueIr, ClassIr, ClassKindIr, ConfigApplicationIr, DartFileIr,
    FunctionIr, MethodIr, MethodParamIr, NameIr, ParamKind, PartIr, SpanIr, SymbolId, TypeIr,
    TypeParamIr,
};
use dust_text::{FileId, TextRange};

use super::*;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(5), TextRange::new(0_u32, 1_u32))
}

fn config(symbol: &str, value: Option<&str>) -> ConfigApplicationIr {
    ConfigApplicationIr::with_arguments(
        SymbolId::new(symbol),
        None,
        value
            .map(|value| vec![AnnotationValueIr::String(value.to_owned())])
            .unwrap_or_default(),
        Default::default(),
        span(),
    )
}

fn annotation(
    symbol: &str,
    value: &str,
    named: impl IntoIterator<Item = (&'static str, AnnotationValueIr)>,
) -> AnnotationIr {
    AnnotationIr {
        raw_name: symbol.to_owned(),
        short_name: symbol.to_owned(),
        prefix: None,
        positional_args: vec![AnnotationValueIr::String(value.to_owned())],
        named_args: named
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
        resolved_symbol: Some(SymbolId::new(symbol)),
        span: span(),
    }
}

fn param(name: &str, extractor: &str) -> MethodParamIr {
    MethodParamIr {
        name: name.to_owned(),
        ty: TypeIr::named("Todo"),
        span: span(),
        kind: ParamKind::Positional,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs: vec![config(extractor, None)],
    }
}

fn method(name: &str, configs: Vec<ConfigApplicationIr>) -> MethodIr {
    MethodIr {
        name: name.to_owned(),
        is_static: false,
        is_external: false,
        return_type: TypeIr::named("Todo"),
        has_body: true,
        body_source: Some("=> todo;".to_owned()),
        params: Vec::new(),
        span: span(),
        traits: Vec::new(),
        configs,
    }
}

fn class(methods: Vec<MethodIr>) -> ClassIr {
    ClassIr {
        kind: ClassKindIr::Class,
        name: "Todos".to_owned(),
        type_parameters: Vec::new(),
        mixins: Vec::new(),
        is_abstract: false,
        is_interface: false,
        superclass_name: None,
        superclass: None,
        span: span(),
        fields: Vec::new(),
        constructors: Vec::new(),
        methods,
        traits: Vec::new(),
        configs: vec![config("dust_server::Controller", Some("/todos"))],
        serde: None,
    }
}

fn file() -> DartFileIr {
    let mut file = DartFileIr::empty(
        ".",
        "dust_test",
        "lib/todos.dart",
        "lib/todos.g.dart",
        span(),
    );
    file.part_directives.push(PartIr {
        uri_literal: None,
        uri: "todos.g.dart".to_owned(),
        span: span(),
    });
    file
}

fn messages(file: &DartFileIr) -> Vec<String> {
    match plan_server(file) {
        Err(diagnostics) => diagnostics.into_iter().map(|value| value.message).collect(),
        Ok(_) => panic!("expected server planning to fail"),
    }
}

#[test]
fn rejects_invalid_method_and_handler_contracts() {
    let mut no_verb = method("missing", Vec::new());
    no_verb.params = vec![param("id", "dust_server::Path")];
    let mut static_method = method(
        "staticRead",
        vec![config("dust_server::GET", Some("/static"))],
    );
    static_method.is_static = true;
    let multiple = method(
        "multiple",
        vec![
            config("dust_server::GET", Some("/multiple")),
            config("dust_server::POST", Some("/multiple")),
        ],
    );
    let mut unsupported = method("text", vec![config("dust_server::GET", Some("/text"))]);
    unsupported.return_type = TypeIr::named("Result");
    let mut library = file();
    library.classes = vec![class(vec![no_verb, static_method, multiple, unsupported])];
    library.classes[0].type_parameters.push(TypeParamIr {
        name: NameIr {
            source: "T".to_owned(),
            short: "T".to_owned(),
            prefix: None,
            span: span(),
        },
        bound: None,
        span: span(),
    });

    let messages = messages(&library);
    for expected in [
        "missing an HTTP verb",
        "concrete instance methods",
        "exactly one HTTP verb",
        "return type is not supported",
        "generic controllers are not supported",
    ] {
        assert!(messages.iter().any(|value| value.contains(expected)));
    }
}

#[test]
fn rejects_top_level_server_metadata_without_a_verb() {
    let mut library = file();
    library.library_annotations = vec![annotation("dust_server::Routes", "/api", [])];
    library.functions = vec![
        FunctionIr {
            name: NameIr {
                source: "health".to_owned(),
                short: "health".to_owned(),
                prefix: None,
                span: span(),
            },
            return_type: TypeIr::named("Health"),
            params: Vec::new(),
            annotations: vec![annotation("dust_server::GET", "/health", [])],
            span: span(),
        },
        FunctionIr {
            name: NameIr {
                source: "missing".to_owned(),
                short: "missing".to_owned(),
                prefix: None,
                span: span(),
            },
            return_type: TypeIr::named("Todo"),
            params: vec![param("id", "dust_server::Path")],
            annotations: Vec::new(),
            span: span(),
        },
        FunctionIr {
            name: NameIr {
                source: "missingMedia".to_owned(),
                short: "missingMedia".to_owned(),
                prefix: None,
                span: span(),
            },
            return_type: TypeIr::named("Todo"),
            params: Vec::new(),
            annotations: vec![annotation("dust_server::FormUrlEncoded", "", [])],
            span: span(),
        },
    ];

    assert_eq!(
        messages(&library)
            .iter()
            .filter(|value| value.as_str() == "server handler is missing an HTTP verb")
            .count(),
        2
    );
}

#[test]
fn rejects_repeated_and_nonfinal_body_extractors() {
    let mut write = method("write", vec![config("dust_server::POST", Some("/"))]);
    write.params = vec![
        param("first", "dust_server::Body"),
        param("second", "dust_server::Body"),
        MethodParamIr {
            name: "limit".to_owned(),
            ty: TypeIr::int(),
            span: span(),
            kind: ParamKind::Positional,
            is_required: true,
            has_default: false,
            default_value_source: None,
            traits: Vec::new(),
            configs: vec![config("dust_server::Query", Some("limit"))],
        },
    ];
    let mut library = file();
    library.classes = vec![class(vec![write])];

    let messages = messages(&library);
    assert!(
        messages
            .iter()
            .any(|value| value.contains("only one request-body source"))
    );
    assert!(
        messages
            .iter()
            .any(|value| value.contains("final handler parameter group"))
    );
}

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

#[test]
fn derives_default_status_from_the_return_type() {
    assert_eq!(default_status(&TypeIr::named("void")), 204);
    assert_eq!(default_status(&TypeIr::named("Unit")), 204);
    assert_eq!(
        default_status(&TypeIr::generic("Future", vec![TypeIr::named("void")],)),
        204
    );
    assert_eq!(default_status(&TypeIr::named("Todo")), 200);
}
