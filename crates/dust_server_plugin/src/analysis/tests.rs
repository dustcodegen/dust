use dust_ir::{MethodParamIr, ParamKind, SpanIr, TypeIr};
use dust_text::{FileId, TextRange};

use super::*;
use crate::plan::{BodyFormat, ParamPlan, RawBodyFormat};

fn span() -> SpanIr {
    SpanIr::new(FileId::new(9), TextRange::new(0_u32, 1_u32))
}

fn param() -> MethodParamIr {
    MethodParamIr {
        name: "value".to_owned(),
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

#[test]
fn names_every_extraction_binding() {
    let values = [
        Extraction::Path("id".to_owned()),
        Extraction::Query("q".to_owned()),
        Extraction::QueryList("tag".to_owned()),
        Extraction::Queries,
        Extraction::Header("x-id".to_owned()),
        Extraction::HeaderMap,
        Extraction::Body(BodyFormat::Json),
        Extraction::Custom("Auth".to_owned()),
        Extraction::RawQuery,
        Extraction::Field("name".to_owned()),
        Extraction::Part {
            name: "file".to_owned(),
            file: true,
        },
        Extraction::RawBody(RawBodyFormat::Bytes),
        Extraction::State,
        Extraction::Peer,
        Extraction::Request,
        Extraction::RequestParts,
    ];
    let names = values
        .iter()
        .map(|value| binding(value).0)
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "path",
            "query",
            "query-list",
            "queries",
            "header",
            "header-map",
            "body",
            "custom",
            "raw-query",
            "field",
            "part",
            "raw-body",
            "state",
            "peer",
            "request",
            "request-parts",
        ]
    );
}

#[test]
fn serializes_every_wrapper_and_unknown_type() {
    let param = param();
    for (wrapper, expected) in [
        (ExtractionWrapper::Required, "required"),
        (ExtractionWrapper::Optional, "optional"),
        (ExtractionWrapper::Fallible, "fallible"),
    ] {
        let fact = param_fact(&ParamPlan {
            param: &param,
            value_type: &param.ty,
            wrapper,
            extraction: Extraction::State,
            validated: false,
            serde_deserialized: false,
        });
        assert_eq!(fact.wrapper, expected);
        assert_eq!(fact.type_source, "String");
    }
    assert_eq!(render_type(&TypeIr::unknown()), "dynamic");
}
