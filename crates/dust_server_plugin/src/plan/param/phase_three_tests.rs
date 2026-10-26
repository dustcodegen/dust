use dust_ir::{
    AnnotationValueIr, ConfigApplicationIr, MethodParamIr, ParamKind, SpanIr, SymbolId, TypeIr,
};
use dust_text::{FileId, TextRange};

use super::*;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(22), TextRange::new(0_u32, 1_u32))
}

fn config(symbol: &str, name: Option<&str>) -> ConfigApplicationIr {
    ConfigApplicationIr::with_arguments(
        SymbolId::new(symbol),
        None,
        name.map(|value| AnnotationValueIr::String(value.to_owned()))
            .into_iter()
            .collect(),
        Default::default(),
        span(),
    )
}

fn param(name: &str, ty: TypeIr, symbol: Option<(&str, Option<&str>)>) -> MethodParamIr {
    MethodParamIr {
        name: name.to_owned(),
        ty,
        span: span(),
        kind: ParamKind::Positional,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs: symbol
            .map(|(symbol, value)| config(symbol, value))
            .into_iter()
            .collect(),
    }
}

fn parse<'a>(
    param: &'a MethodParamIr,
    media: BodyMedia,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParamPlan<'a> {
    parse_param(param, &BTreeSet::new(), media, &[], &[], diagnostics).unwrap()
}

#[test]
fn parses_every_phase_three_extractor_shape() {
    let cases = [
        param(
            "raw",
            TypeIr::string().nullable(),
            Some(("dust_server::RawQuery", None)),
        ),
        param(
            "field",
            TypeIr::string(),
            Some(("dust_server::Field", Some("title"))),
        ),
        param(
            "part",
            TypeIr::named("UploadedFile"),
            Some(("dust_server::Part", Some("file"))),
        ),
        param(
            "bytes",
            TypeIr::named("Uint8List"),
            Some(("dust_server::RawBody", None)),
        ),
        param(
            "text",
            TypeIr::string(),
            Some(("dust_server::RawBody", None)),
        ),
        param(
            "stream",
            TypeIr::generic("Stream", vec![TypeIr::list_of(TypeIr::int())]),
            Some(("dust_server::RawBody", None)),
        ),
        param(
            "state",
            TypeIr::named("TodoState"),
            Some(("dust_server::State", None)),
        ),
        param(
            "peer",
            TypeIr::named("PeerInfo"),
            Some(("dust_server::Peer", None)),
        ),
        param("request", TypeIr::named("Request"), None),
        param("parts", TypeIr::named("RequestParts"), None),
        param(
            "body",
            TypeIr::named("CreateTodo"),
            Some(("dust_server::Body", None)),
        ),
    ];
    let media = [
        BodyMedia::Json,
        BodyMedia::Form,
        BodyMedia::Multipart,
        BodyMedia::Json,
        BodyMedia::Json,
        BodyMedia::Json,
        BodyMedia::Json,
        BodyMedia::Json,
        BodyMedia::Json,
        BodyMedia::Json,
        BodyMedia::Form,
    ];
    let mut diagnostics = Vec::new();
    let plans = cases
        .iter()
        .zip(media)
        .map(|(param, media)| parse(param, media, &mut diagnostics))
        .collect::<Vec<_>>();

    assert!(matches!(plans[0].extraction, Extraction::RawQuery));
    assert!(matches!(plans[1].extraction, Extraction::Field(_)));
    assert!(matches!(
        plans[2].extraction,
        Extraction::Part { file: true, .. }
    ));
    assert!(matches!(
        plans[3].extraction,
        Extraction::RawBody(RawBodyFormat::Bytes)
    ));
    assert!(matches!(
        plans[4].extraction,
        Extraction::RawBody(RawBodyFormat::Text)
    ));
    assert!(matches!(
        plans[5].extraction,
        Extraction::RawBody(RawBodyFormat::Stream)
    ));
    assert!(matches!(plans[6].extraction, Extraction::State));
    assert!(matches!(plans[7].extraction, Extraction::Peer));
    assert!(matches!(plans[8].extraction, Extraction::Request));
    assert!(matches!(plans[9].extraction, Extraction::RequestParts));
    assert!(matches!(
        plans[10].extraction,
        Extraction::Body(BodyFormat::Form)
    ));
    assert!(diagnostics.is_empty());
}

#[test]
fn rejects_invalid_phase_three_extractor_contracts() {
    let cases = [
        param(
            "raw",
            TypeIr::string(),
            Some(("dust_server::RawQuery", None)),
        ),
        param(
            "field",
            TypeIr::named("Todo"),
            Some(("dust_server::Field", Some("title"))),
        ),
        param(
            "part",
            TypeIr::string(),
            Some(("dust_server::Part", Some("part"))),
        ),
        param(
            "body",
            TypeIr::named("Todo"),
            Some(("dust_server::Body", None)),
        ),
        param(
            "rawBody",
            TypeIr::int(),
            Some(("dust_server::RawBody", None)),
        ),
        param(
            "state",
            TypeIr::named("TodoState").nullable(),
            Some(("dust_server::State", None)),
        ),
        param("peer", TypeIr::string(), Some(("dust_server::Peer", None))),
        param("request", TypeIr::named("Request").nullable(), None),
    ];
    let media = [
        BodyMedia::Json,
        BodyMedia::Json,
        BodyMedia::Form,
        BodyMedia::Multipart,
        BodyMedia::Form,
        BodyMedia::Json,
        BodyMedia::Json,
        BodyMedia::Json,
    ];
    let mut diagnostics = Vec::new();
    for (param, media) in cases.iter().zip(media) {
        let _ = parse_param(param, &BTreeSet::new(), media, &[], &[], &mut diagnostics);
    }

    assert_eq!(diagnostics.len(), 10);
    for expected in [
        "RawQuery requires String?",
        "cannot be coerced",
        "Part requires MultiPart",
        "Body is not supported with MultiPart",
        "RawBody requires",
        "RawBody cannot be combined",
        "State requires",
        "Peer requires PeerInfo",
        "require exactly one server extractor",
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|value| value.message.contains(expected)),
            "missing {expected}"
        );
    }
}
