use dust_ir::{
    AnnotationValueIr, ConfigApplicationIr, MethodParamIr, ParamKind, SpanIr, SymbolId, TypeIr,
};
use dust_text::{FileId, TextRange};

use super::*;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(3), TextRange::new(0_u32, 1_u32))
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

fn param(name: &str, ty: TypeIr, configs: Vec<ConfigApplicationIr>) -> MethodParamIr {
    MethodParamIr {
        name: name.to_owned(),
        ty,
        span: span(),
        kind: ParamKind::Positional,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs,
    }
}

#[test]
fn parses_all_phase_one_extractor_shapes() {
    let placeholders = BTreeSet::from(["id".to_owned()]);
    let cases = [
        param(
            "id",
            TypeIr::string(),
            vec![config("dust_server::Path", None)],
        ),
        param(
            "tags",
            TypeIr::list_of(TypeIr::string()),
            vec![config("dust_server::Query", Some("tag"))],
        ),
        param(
            "queries",
            TypeIr::map_of(TypeIr::string(), TypeIr::string()),
            vec![config("dust_server::Queries", None)],
        ),
        param(
            "token",
            TypeIr::string(),
            vec![config("dust_server::Header", Some("authorization"))],
        ),
        param(
            "headers",
            TypeIr::map_of(TypeIr::string(), TypeIr::string()),
            vec![config("dust_server::HeaderMap", None)],
        ),
        param(
            "body",
            TypeIr::named("Todo"),
            vec![config("dust_server::Body", None)],
        ),
    ];
    let mut diagnostics = Vec::new();
    let plans = cases
        .iter()
        .map(|param| parse_param(param, &placeholders, &mut diagnostics).unwrap())
        .collect::<Vec<_>>();
    assert!(matches!(plans[0].extraction, Extraction::Path(_)));
    assert!(matches!(plans[1].extraction, Extraction::QueryList(_)));
    assert!(matches!(plans[2].extraction, Extraction::Queries));
    assert!(matches!(plans[3].extraction, Extraction::Header(_)));
    assert!(matches!(plans[4].extraction, Extraction::HeaderMap));
    assert!(matches!(plans[5].extraction, Extraction::Body));
    assert!(diagnostics.is_empty());
}

#[test]
fn rejects_missing_multiple_and_invalid_extractor_shapes() {
    let placeholders = BTreeSet::new();
    let cases = [
        param("id", TypeIr::string(), Vec::new()),
        param(
            "id",
            TypeIr::string(),
            vec![
                config("dust_server::Path", None),
                config("dust_server::Query", Some("id")),
            ],
        ),
        param(
            "headers",
            TypeIr::string(),
            vec![config("dust_server::HeaderMap", None)],
        ),
    ];
    let mut diagnostics = Vec::new();
    for param in &cases {
        let _ = parse_param(param, &placeholders, &mut diagnostics);
    }
    assert_eq!(diagnostics.len(), 3);
}
