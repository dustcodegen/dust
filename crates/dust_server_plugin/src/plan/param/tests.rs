use dust_ir::{
    AnnotationValueIr, ConfigApplicationIr, MethodParamIr, NameIr, ParamKind, SpanIr, SymbolId,
    TypeIr,
};
use dust_text::{FileId, TextRange};

use super::*;
use crate::plan::{BodyFormat, body::BodyMedia};

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

fn extract_config(value: AnnotationValueIr) -> ConfigApplicationIr {
    ConfigApplicationIr::with_arguments(
        SymbolId::new("dust_server::Extract"),
        None,
        vec![value],
        Default::default(),
        span(),
    )
}

fn member(source: &str) -> AnnotationValueIr {
    let (prefix, short) = source
        .rsplit_once('.')
        .map(|(prefix, short)| (Some(prefix.to_owned()), short.to_owned()))
        .unwrap_or_else(|| (None, source.to_owned()));
    AnnotationValueIr::Member(NameIr {
        source: source.to_owned(),
        short,
        prefix,
        span: span(),
    })
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
        .map(|param| {
            parse_param(param, &placeholders, BodyMedia::Json, &[], &mut diagnostics).unwrap()
        })
        .collect::<Vec<_>>();
    assert!(matches!(plans[0].extraction, Extraction::Path(_)));
    assert!(matches!(plans[1].extraction, Extraction::QueryList(_)));
    assert!(matches!(plans[2].extraction, Extraction::Queries));
    assert!(matches!(plans[3].extraction, Extraction::Header(_)));
    assert!(matches!(plans[4].extraction, Extraction::HeaderMap));
    assert!(matches!(
        plans[5].extraction,
        Extraction::Body(BodyFormat::Json)
    ));
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
        param(
            "id",
            TypeIr::string(),
            vec![config("dust_server::Path", None)],
        ),
    ];
    let mut diagnostics = Vec::new();
    for param in &cases {
        let _ = parse_param(param, &placeholders, BodyMedia::Json, &[], &mut diagnostics);
    }
    assert_eq!(diagnostics.len(), 4);
}

#[test]
fn parses_custom_extractors_and_failure_wrappers() {
    let cases = [
        param(
            "user",
            TypeIr::named("AuthUser"),
            vec![extract_config(member("auth.BearerAuth"))],
        ),
        param(
            "viewer",
            TypeIr::generic("Option", vec![TypeIr::named("AuthUser")]),
            vec![extract_config(member("BearerAuth"))],
        ),
        param(
            "outcome",
            TypeIr::generic(
                "Result",
                vec![TypeIr::named("AuthUser"), TypeIr::named("Rejection")],
            ),
            vec![extract_config(member("TodosWrite"))],
        ),
        param(
            "id",
            TypeIr::generic("Option", vec![TypeIr::string()]),
            vec![config("dust_server::Query", Some("id"))],
        ),
    ];
    let mut diagnostics = Vec::new();
    let plans = cases
        .iter()
        .map(|param| {
            parse_param(
                param,
                &BTreeSet::new(),
                BodyMedia::Json,
                &[],
                &mut diagnostics,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    assert!(matches!(
        &plans[0].extraction,
        Extraction::Custom(name) if name == "auth.BearerAuth"
    ));
    assert_eq!(plans[0].wrapper, ExtractionWrapper::Required);
    assert_eq!(plans[1].wrapper, ExtractionWrapper::Optional);
    assert_eq!(plans[2].wrapper, ExtractionWrapper::Fallible);
    assert!(plans[3].value_type.is_named("String"));
    assert!(matches!(plans[3].extraction, Extraction::Query(_)));
    assert!(diagnostics.is_empty());
}

#[test]
fn rejects_invalid_custom_values_and_wrapper_shapes() {
    let cases = [
        param(
            "user",
            TypeIr::named("AuthUser"),
            vec![extract_config(AnnotationValueIr::String(
                "BearerAuth".to_owned(),
            ))],
        ),
        param(
            "missing",
            TypeIr::named("AuthUser"),
            vec![config("dust_server::Extract", None)],
        ),
        param(
            "viewer",
            TypeIr::named("Option"),
            vec![extract_config(member("BearerAuth"))],
        ),
        param(
            "outcome",
            TypeIr::generic(
                "Result",
                vec![TypeIr::named("AuthUser"), TypeIr::named("OtherError")],
            ),
            vec![extract_config(member("BearerAuth"))],
        ),
    ];
    let mut diagnostics = Vec::new();
    for param in &cases {
        assert!(
            parse_param(
                param,
                &BTreeSet::new(),
                BodyMedia::Json,
                &[],
                &mut diagnostics,
            )
            .is_none()
        );
    }

    assert_eq!(diagnostics.len(), 4);
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("type reference"))
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|value| value.message.contains("optional or fallible"))
            .count(),
        2
    );
}

#[test]
fn rejects_missing_builtin_arguments_and_invalid_raw_query_type() {
    let cases = [
        param(
            "query",
            TypeIr::string(),
            vec![config("dust_server::Query", None)],
        ),
        param(
            "header",
            TypeIr::string(),
            vec![config("dust_server::Header", None)],
        ),
        param(
            "raw",
            TypeIr::string(),
            vec![config("dust_server::RawQuery", None)],
        ),
    ];
    let mut diagnostics = Vec::new();
    for param in &cases[..2] {
        assert!(
            parse_param(
                param,
                &BTreeSet::new(),
                BodyMedia::Json,
                &[],
                &mut diagnostics,
            )
            .is_none()
        );
    }
    assert!(
        parse_param(
            &cases[2],
            &BTreeSet::new(),
            BodyMedia::Json,
            &[],
            &mut diagnostics,
        )
        .is_some()
    );

    assert_eq!(diagnostics.len(), 3);
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("query name"))
    );
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("header name"))
    );
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("RawQuery requires String?"))
    );
}

#[test]
fn rejects_an_unknown_server_extractor() {
    let param = param(
        "cookie",
        TypeIr::string(),
        vec![config("dust_server::Cookie", None)],
    );
    let mut diagnostics = Vec::new();

    assert!(
        parse_param(
            &param,
            &BTreeSet::new(),
            BodyMedia::Json,
            &[],
            &mut diagnostics,
        )
        .is_none()
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message,
        "extractor is not supported by generated server handlers yet"
    );
}
