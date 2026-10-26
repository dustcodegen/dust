use std::collections::BTreeMap;

use dust_ir::{
    AnnotationIr, AnnotationValueIr, ConfigApplicationIr, FunctionIr, MethodIr, MethodParamIr,
    NameIr, ParamKind, SpanIr, SymbolId, TypeIr,
};
use dust_text::{FileId, TextRange};

use super::*;
use crate::plan::{BodyFormat, ExtractionWrapper, RawBodyFormat};

fn span() -> SpanIr {
    SpanIr::new(FileId::new(21), TextRange::new(0_u32, 1_u32))
}

fn annotation(symbol: &str, positional_args: Vec<AnnotationValueIr>) -> AnnotationIr {
    AnnotationIr {
        raw_name: symbol.to_owned(),
        short_name: symbol.rsplit("::").next().unwrap().to_owned(),
        prefix: None,
        positional_args,
        named_args: BTreeMap::new(),
        resolved_symbol: Some(SymbolId::new(symbol)),
        span: span(),
    }
}

fn function(annotations: Vec<AnnotationIr>) -> FunctionIr {
    FunctionIr {
        name: NameIr {
            source: "write".to_owned(),
            short: "write".to_owned(),
            prefix: None,
            span: span(),
        },
        return_type: TypeIr::named("void"),
        params: Vec::new(),
        annotations,
        span: span(),
    }
}

fn method(configs: Vec<ConfigApplicationIr>) -> MethodIr {
    MethodIr {
        name: "write".to_owned(),
        is_static: false,
        is_external: false,
        return_type: TypeIr::named("void"),
        has_body: true,
        body_source: Some("{}".to_owned()),
        params: Vec::new(),
        span: span(),
        traits: Vec::new(),
        configs,
    }
}

fn param(name: &str) -> MethodParamIr {
    MethodParamIr {
        name: name.to_owned(),
        ty: TypeIr::string(),
        span: span(),
        kind: ParamKind::Positional,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs: Vec::new(),
    }
}

fn plans<'a>(
    params: &'a [MethodParamIr],
    extractions: impl IntoIterator<Item = Extraction>,
) -> Vec<ParamPlan<'a>> {
    params
        .iter()
        .zip(extractions)
        .map(|(param, extraction)| ParamPlan {
            param,
            value_type: &param.ty,
            wrapper: ExtractionWrapper::Required,
            extraction,
            validated: false,
            serde_deserialized: false,
        })
        .collect()
}

#[test]
fn parses_media_markers_and_rejects_invalid_declarations() {
    let plain = function(Vec::new());
    let mut diagnostics = Vec::new();
    assert_eq!(
        parse_body_media(Callable::Function(&plain), &mut diagnostics),
        BodyMedia::Json
    );

    let form = function(vec![annotation("dust_server::FormUrlEncoded", Vec::new())]);
    assert_eq!(
        parse_body_media(Callable::Function(&form), &mut diagnostics),
        BodyMedia::Form
    );

    let invalid = function(vec![
        annotation(
            "dust_server::FormUrlEncoded",
            vec![AnnotationValueIr::String("unexpected".to_owned())],
        ),
        annotation("dust_server::MultiPart", Vec::new()),
    ]);
    assert_eq!(
        parse_body_media(Callable::Function(&invalid), &mut diagnostics),
        BodyMedia::Form
    );
    assert_eq!(diagnostics.len(), 2);

    let multipart = method(vec![ConfigApplicationIr::with_arguments(
        SymbolId::new("dust_server::MultiPart"),
        None,
        Vec::new(),
        BTreeMap::new(),
        span(),
    )]);
    assert_eq!(
        parse_body_media(Callable::Method(&multipart), &mut diagnostics),
        BodyMedia::Multipart
    );
}

#[test]
fn accepts_one_shared_body_group_for_each_media() {
    let callable = function(Vec::new());
    let params = [param("first"), param("second")];
    let mut diagnostics = Vec::new();
    let fields = plans(
        &params,
        [
            Extraction::Field("first".to_owned()),
            Extraction::Field("second".to_owned()),
        ],
    );
    validate_body_order(
        Callable::Function(&callable),
        BodyMedia::Form,
        &fields,
        &mut diagnostics,
    );
    let parts = plans(
        &params,
        [
            Extraction::Part {
                name: "first".to_owned(),
                file: false,
            },
            Extraction::Part {
                name: "second".to_owned(),
                file: true,
            },
        ],
    );
    validate_body_order(
        Callable::Function(&callable),
        BodyMedia::Multipart,
        &parts,
        &mut diagnostics,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn rejects_multiple_nonfinal_and_media_less_body_groups() {
    let callable = function(Vec::new());
    let params = [param("body"), param("tail")];
    let mut diagnostics = Vec::new();
    let multiple = plans(
        &params,
        [
            Extraction::Body(BodyFormat::Json),
            Extraction::RawBody(RawBodyFormat::Text),
        ],
    );
    validate_body_order(
        Callable::Function(&callable),
        BodyMedia::Json,
        &multiple,
        &mut diagnostics,
    );
    let nonfinal = plans(
        &params,
        [
            Extraction::Body(BodyFormat::Json),
            Extraction::Query("tail".to_owned()),
        ],
    );
    validate_body_order(
        Callable::Function(&callable),
        BodyMedia::Json,
        &nonfinal,
        &mut diagnostics,
    );
    validate_body_order(
        Callable::Function(&callable),
        BodyMedia::Multipart,
        &[],
        &mut diagnostics,
    );
    assert_eq!(diagnostics.len(), 3);
}
