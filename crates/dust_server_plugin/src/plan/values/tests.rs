use std::collections::BTreeMap;

use dust_ir::{
    AnnotationNumberKindIr, AnnotationValueIr, ConfigApplicationIr, ExprSourceIr, NameIr, SpanIr,
    SymbolId,
};
use dust_text::{FileId, TextRange};

use super::*;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(7), TextRange::new(2_u32, 9_u32))
}

fn config(
    positional: Vec<AnnotationValueIr>,
    named: impl IntoIterator<Item = (&'static str, AnnotationValueIr)>,
) -> ConfigApplicationIr {
    ConfigApplicationIr::with_arguments(
        SymbolId::new("dust_server::GET"),
        None,
        positional,
        named
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
        span(),
    )
}

fn int(value: &str) -> AnnotationValueIr {
    AnnotationValueIr::Number {
        source: value.to_owned(),
        kind: AnnotationNumberKindIr::Int,
    }
}

fn name(value: &str) -> NameIr {
    NameIr {
        source: value.to_owned(),
        short: value.to_owned(),
        prefix: None,
        span: span(),
    }
}

#[test]
fn status_validation_covers_absent_valid_wrong_shape_and_range() {
    let mut diagnostics = Vec::new();
    let absent = config(Vec::new(), []);
    assert_eq!(parse_status(&Args::Config(&absent), &mut diagnostics), None);
    let valid = config(Vec::new(), [("status", int("201"))]);
    assert_eq!(
        parse_status(&Args::Config(&valid), &mut diagnostics),
        Some(201)
    );
    let wrong = config(
        Vec::new(),
        [("status", AnnotationValueIr::String("ok".to_owned()))],
    );
    assert_eq!(parse_status(&Args::Config(&wrong), &mut diagnostics), None);
    let range = config(Vec::new(), [("status", int("999"))]);
    assert_eq!(parse_status(&Args::Config(&range), &mut diagnostics), None);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn named_argument_validation_accepts_contract_and_rejects_wrong_shapes() {
    let valid = config(
        Vec::new(),
        [
            ("summary", AnnotationValueIr::Null),
            ("description", AnnotationValueIr::String("read".to_owned())),
            (
                "operationId",
                AnnotationValueIr::String("readTodo".to_owned()),
            ),
            ("deprecated", AnnotationValueIr::Bool(false)),
            ("hidden", AnnotationValueIr::Bool(true)),
            (
                "tags",
                AnnotationValueIr::List(vec![AnnotationValueIr::String("todo".to_owned())]),
            ),
            ("middleware", AnnotationValueIr::List(Vec::new())),
        ],
    );
    let mut diagnostics = Vec::new();
    validate_named(
        &Args::Config(&valid),
        &[
            "summary",
            "description",
            "operationId",
            "deprecated",
            "hidden",
            "tags",
            "middleware",
        ],
        &mut diagnostics,
    );
    assert!(diagnostics.is_empty());

    let invalid = config(
        Vec::new(),
        [
            ("unknown", AnnotationValueIr::Null),
            ("summary", AnnotationValueIr::Bool(true)),
            ("deprecated", AnnotationValueIr::String("no".to_owned())),
            (
                "tags",
                AnnotationValueIr::List(vec![AnnotationValueIr::Bool(true)]),
            ),
            ("middleware", AnnotationValueIr::Bool(false)),
        ],
    );
    validate_named(&Args::Config(&invalid), &[], &mut diagnostics);
    assert_eq!(diagnostics.len(), 9);
}

#[test]
fn middleware_requires_emit_safe_const_layers() {
    let valid = config(
        Vec::new(),
        [(
            "middleware",
            AnnotationValueIr::List(vec![
                AnnotationValueIr::Constructor {
                    name: name("TraceLayer"),
                    positional_args: vec![
                        AnnotationValueIr::List(vec![AnnotationValueIr::Bool(true)]),
                        AnnotationValueIr::Set(vec![AnnotationValueIr::Null]),
                        AnnotationValueIr::Map(vec![(
                            AnnotationValueIr::String("limit".to_owned()),
                            int("2"),
                        )]),
                        AnnotationValueIr::Record(vec![(
                            "label".to_owned(),
                            AnnotationValueIr::String("controller".to_owned()),
                        )]),
                    ],
                    named_args: BTreeMap::new(),
                },
                AnnotationValueIr::Member(name("sharedLayer")),
            ]),
        )],
    );
    let mut diagnostics = Vec::new();
    assert_eq!(
        parse_middleware(&Args::Config(&valid), &mut diagnostics).len(),
        2
    );
    assert!(diagnostics.is_empty());

    let invalid = config(
        Vec::new(),
        [(
            "middleware",
            AnnotationValueIr::List(vec![
                AnnotationValueIr::String("not a layer".to_owned()),
                AnnotationValueIr::Constructor {
                    name: name("TraceLayer"),
                    positional_args: vec![AnnotationValueIr::Expression(ExprSourceIr {
                        source: "buildLabel()".to_owned(),
                        span: span(),
                    })],
                    named_args: BTreeMap::new(),
                },
            ]),
        )],
    );
    assert!(parse_middleware(&Args::Config(&invalid), &mut diagnostics).is_empty());
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn metadata_value_helpers_preserve_order_and_types() {
    let value = config(
        Vec::new(),
        [
            (
                "tags",
                AnnotationValueIr::List(vec![
                    AnnotationValueIr::String("group".to_owned()),
                    AnnotationValueIr::String("route".to_owned()),
                ]),
            ),
            ("deprecated", AnnotationValueIr::Bool(true)),
        ],
    );

    assert_eq!(
        parse_tags(&Args::Config(&value)),
        vec!["group".to_owned(), "route".to_owned()]
    );
    assert_eq!(
        metadata::bool_value(value.named_args.get("deprecated").unwrap()),
        Some(true)
    );
    assert_eq!(metadata::bool_value(&AnnotationValueIr::Null), None);
    assert!(parse_tags(&Args::Config(&config(Vec::new(), []))).is_empty());
}

#[test]
fn positional_string_helpers_cover_all_shapes() {
    let mut diagnostics = Vec::new();
    let value = config(vec![AnnotationValueIr::String("/todos".to_owned())], []);
    assert_eq!(
        required_string(&Args::Config(&value), 0, "path", &mut diagnostics),
        Some("/todos".to_owned())
    );
    assert_eq!(
        optional_string(&Args::Config(&value), 0, &mut diagnostics),
        Some("/todos".to_owned())
    );
    let missing = config(Vec::new(), []);
    assert_eq!(
        required_string(&Args::Config(&missing), 0, "path", &mut diagnostics),
        None
    );
    assert_eq!(
        optional_string(&Args::Config(&missing), 0, &mut diagnostics),
        None
    );
    let null = config(vec![AnnotationValueIr::Null], []);
    assert_eq!(
        optional_string(&Args::Config(&null), 0, &mut diagnostics),
        None
    );
    let wrong = config(vec![AnnotationValueIr::Bool(true)], []);
    assert_eq!(
        required_string(&Args::Config(&wrong), 0, "path", &mut diagnostics),
        None
    );
    assert_eq!(
        optional_string(&Args::Config(&wrong), 0, &mut diagnostics),
        None
    );
    let extra = config(
        vec![
            AnnotationValueIr::String("a".to_owned()),
            AnnotationValueIr::String("b".to_owned()),
        ],
        [],
    );
    assert_eq!(
        optional_string(&Args::Config(&extra), 0, &mut diagnostics),
        Some("a".to_owned())
    );
    assert_eq!(diagnostics.len(), 4);
}

#[test]
fn zero_argument_helpers_and_paths_report_exact_boundaries() {
    let with_args = config(
        vec![AnnotationValueIr::String("x".to_owned())],
        [("name", AnnotationValueIr::String("y".to_owned()))],
    );
    let mut diagnostics = Vec::new();
    validate_no_args(&Args::Config(&with_args), &mut diagnostics);
    validate_zero_named(&Args::Config(&with_args), &mut diagnostics);
    assert_eq!(diagnostics.len(), 2);

    for valid in ["/", "/todos/{id}", "/todos/{id|[0-9]+}"] {
        validate_route_path(valid, span(), &mut diagnostics);
    }
    assert_eq!(
        path_keys("/todos/{id|[0-9]+}"),
        BTreeSet::from(["id".to_owned()])
    );
    for invalid in [
        "todos",
        "/todos?x=1",
        "/todos#x",
        "/{",
        "/{}",
        "/{a/b}",
        "/x}",
        "/x}/{id}",
    ] {
        validate_route_path(invalid, span(), &mut diagnostics);
    }
    assert_eq!(diagnostics.len(), 10);
}

#[test]
fn annotation_args_support_general_annotations() {
    let annotation = dust_ir::AnnotationIr {
        raw_name: "Routes".to_owned(),
        short_name: "Routes".to_owned(),
        prefix: None,
        positional_args: vec![AnnotationValueIr::String("/notes".to_owned())],
        named_args: BTreeMap::new(),
        resolved_symbol: Some(SymbolId::new("dust_server::Routes")),
        span: span(),
    };
    let args = Args::Annotation(&annotation);
    assert_eq!(args.span(), span());
    assert!(args.named().is_empty());
    assert_eq!(
        string_value(&args.positional()[0]),
        Some("/notes".to_owned())
    );
    assert_eq!(string_value(&AnnotationValueIr::Null), None);
}
