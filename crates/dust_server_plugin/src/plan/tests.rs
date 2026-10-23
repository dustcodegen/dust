use std::collections::BTreeMap;

use dust_ir::{
    AnnotationIr, AnnotationValueIr, ClassIr, ClassKindIr, ConfigApplicationIr, DartFileIr,
    FunctionIr, MethodIr, MethodParamIr, NameIr, ParamKind, PartIr, SpanIr, SymbolId, TypeIr,
    TypeParamIr,
};
use dust_text::{FileId, TextRange};

use super::*;

mod routes;

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
