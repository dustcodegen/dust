use dust_ir::{
    AnnotationIr, ConfigApplicationIr, DartFileIr, FunctionIr, MethodParamIr, NameIr, PartIr,
    SpanIr, SymbolId, TypeIr,
};
use dust_text::{FileId, TextRange};

use super::*;
use crate::plan::{Callable, ControllerPlan, RouteMetadataPlan};

fn span() -> SpanIr {
    SpanIr::new(FileId::new(2), TextRange::new(0_u32, 1_u32))
}

fn file() -> DartFileIr {
    DartFileIr::empty(
        ".",
        "dust_test",
        "lib/todo_routes.dart",
        "lib/todo_routes.g.dart",
        span(),
    )
}

fn config(symbol: &str) -> ConfigApplicationIr {
    ConfigApplicationIr::new(SymbolId::new(symbol), None, span())
}

fn annotation(symbol: &str) -> AnnotationIr {
    AnnotationIr {
        raw_name: symbol.to_owned(),
        short_name: symbol.to_owned(),
        prefix: None,
        positional_args: Vec::new(),
        named_args: Default::default(),
        resolved_symbol: Some(SymbolId::new(symbol)),
        span: span(),
    }
}

fn function() -> FunctionIr {
    FunctionIr {
        name: NameIr {
            source: "read".to_owned(),
            short: "read".to_owned(),
            prefix: None,
            span: span(),
        },
        return_type: TypeIr::named("Todo"),
        params: Vec::new(),
        annotations: Vec::new(),
        span: span(),
    }
}

#[test]
fn validates_part_and_default_router_names() {
    let mut diagnostics = Vec::new();
    let mut library = file();
    validate_part(&library, &mut diagnostics);
    assert_eq!(diagnostics.len(), 1);
    library.part_directives.push(PartIr {
        uri_literal: None,
        uri: "todo_routes.g.dart".to_owned(),
        span: span(),
    });
    validate_part(&library, &mut diagnostics);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        default_routes_name("lib/user_profiles.dart"),
        "userProfileRoutes"
    );
    assert_eq!(
        default_routes_name("lib/user-profiles.dart"),
        "userProfileRoutes"
    );
    assert_eq!(default_routes_name(""), "serverRoutes");
}

#[test]
fn rejects_duplicate_routes_per_group() {
    let function = function();
    let handlers = vec![
        HandlerPlan {
            callable: Callable::Function(&function),
            method: "GET",
            path: "/".to_owned(),
            status: 200,
            params: Vec::new(),
            middleware: Vec::new(),
            metadata: RouteMetadataPlan {
                operation_id: "read".to_owned(),
                summary: None,
                description: None,
                tags: Vec::new(),
                deprecated: false,
                hidden: false,
            },
        },
        HandlerPlan {
            callable: Callable::Function(&function),
            method: "GET",
            path: "/".to_owned(),
            status: 200,
            params: Vec::new(),
            middleware: Vec::new(),
            metadata: RouteMetadataPlan {
                operation_id: "read".to_owned(),
                summary: None,
                description: None,
                tags: Vec::new(),
                deprecated: false,
                hidden: false,
            },
        },
    ];
    let controllers: Vec<ControllerPlan<'_>> = Vec::new();
    let mut diagnostics = Vec::new();
    validate_unique_routes(&controllers, &handlers, &mut diagnostics);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn resolves_config_verb_and_server_ownership() {
    let configs = [config("dust_server::GET"), config("dust_dart::GET")];
    assert_eq!(verb_config(&configs[0]), Some("GET"));
    assert_eq!(verb_config(&configs[1]), None);
    assert!(is_server_config(&configs[0]));
    assert!(!is_server_config(&configs[1]));
    for (symbol, method) in VERBS {
        assert_eq!(verb_config(&config(symbol)), Some(*method));
    }
}

#[test]
fn finds_server_spans_at_every_supported_coordinate() {
    let mut library = file();
    library
        .library_annotations
        .push(annotation("dust_server::Routes"));
    assert_eq!(first_server_span(&library), Some(span()));

    library.library_annotations.clear();
    let mut function = function();
    function.annotations.push(annotation("dust_server::GET"));
    library.functions = vec![function];
    assert_eq!(first_server_span(&library), Some(span()));

    library.functions[0].annotations.clear();
    library.functions[0].params.push(MethodParamIr {
        name: "id".to_owned(),
        ty: TypeIr::string(),
        span: span(),
        kind: dust_ir::ParamKind::Positional,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs: vec![config("dust_server::Path")],
    });
    assert_eq!(first_server_span(&library), Some(span()));

    library.functions.clear();
    assert_eq!(first_server_span(&library), None);
}
