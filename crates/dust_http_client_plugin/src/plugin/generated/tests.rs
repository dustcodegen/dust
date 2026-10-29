use dust_ir::{MethodIr, MethodParamIr, ParamKind, SpanIr, TypeIr};
use dust_text::{FileId, TextRange};

use super::*;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(1), TextRange::new(0_u32, 1_u32))
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

fn endpoint(
    name: &str,
    verb: GeneratedHttpVerb,
    request_mode: GeneratedRequestMode,
    params: Vec<MethodParamIr>,
    bindings: Vec<GeneratedParam>,
) -> GeneratedEndpoint {
    GeneratedEndpoint {
        method: MethodIr {
            name: name.to_owned(),
            is_static: false,
            is_external: false,
            return_type: TypeIr::generic("Future", vec![TypeIr::string()]),
            has_body: false,
            body_source: None,
            params,
            span: span(),
            traits: Vec::new(),
            configs: Vec::new(),
        },
        verb,
        path: format!("/{name}"),
        request_mode,
        params: bindings,
    }
}

#[test]
fn standalone_generation_covers_every_binding_verb_and_request_mode() {
    let wire_params = [
        "path",
        "query",
        "queries",
        "queryModel",
        "header",
        "headers",
        "body",
        "field",
        "part",
    ]
    .into_iter()
    .map(param)
    .collect::<Vec<_>>();
    let bindings = vec![
        GeneratedParam::Path {
            index: 0,
            key: "id".to_owned(),
        },
        GeneratedParam::Query {
            index: 1,
            key: "q".to_owned(),
        },
        GeneratedParam::Queries { index: 2 },
        GeneratedParam::QueryModel { index: 3 },
        GeneratedParam::Header {
            index: 4,
            key: "x-token".to_owned(),
        },
        GeneratedParam::HeaderMap { index: 5 },
        GeneratedParam::Body { index: 6 },
        GeneratedParam::Field {
            index: 7,
            key: "title".to_owned(),
        },
        GeneratedParam::Part {
            index: 8,
            key: "file".to_owned(),
        },
    ];
    let endpoints = vec![
        endpoint(
            "getAll",
            GeneratedHttpVerb::Get,
            GeneratedRequestMode::Standard,
            wire_params,
            bindings,
        ),
        endpoint(
            "postForm",
            GeneratedHttpVerb::Post,
            GeneratedRequestMode::FormUrlEncoded,
            vec![param("field")],
            vec![GeneratedParam::Field {
                index: 0,
                key: "field".to_owned(),
            }],
        ),
        endpoint(
            "putPart",
            GeneratedHttpVerb::Put,
            GeneratedRequestMode::MultiPart,
            vec![param("part")],
            vec![GeneratedParam::Part {
                index: 0,
                key: "part".to_owned(),
            }],
        ),
        endpoint(
            "patch",
            GeneratedHttpVerb::Patch,
            Default::default(),
            Vec::new(),
            Vec::new(),
        ),
        endpoint(
            "delete",
            GeneratedHttpVerb::Delete,
            Default::default(),
            Vec::new(),
            Vec::new(),
        ),
        endpoint(
            "head",
            GeneratedHttpVerb::Head,
            Default::default(),
            Vec::new(),
            Vec::new(),
        ),
        endpoint(
            "options",
            GeneratedHttpVerb::Options,
            Default::default(),
            Vec::new(),
            Vec::new(),
        ),
    ];

    let source = generate_client_library(&GeneratedClient {
        class_name: "EverythingClient".to_owned(),
        imports: vec!["import 'package:models/models.dart' as models;".to_owned()],
        endpoints,
    })
    .unwrap();

    assert!(source.starts_with(GENERATED_HEADER));
    for method in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
        assert!(source.contains(&format!("method: '{method}'")));
    }
    assert!(source.contains("FormData.fromMap"));
    assert!(source.contains("application/x-www-form-urlencoded"));
}

#[test]
fn standalone_generation_rejects_unsupported_returns_and_binding_indexes() {
    let mut unsupported = endpoint(
        "unsupported",
        GeneratedHttpVerb::Get,
        Default::default(),
        Vec::new(),
        Vec::new(),
    );
    unsupported.method.return_type = TypeIr::function("void Function()");
    let error = generate_client_library(&GeneratedClient {
        class_name: "InvalidClient".to_owned(),
        imports: Vec::new(),
        endpoints: vec![unsupported],
    })
    .unwrap_err();
    assert!(error.message.contains("unsupported return type"));

    let error = generate_client_library(&GeneratedClient {
        class_name: "InvalidClient".to_owned(),
        imports: Vec::new(),
        endpoints: vec![endpoint(
            "invalidIndex",
            GeneratedHttpVerb::Get,
            Default::default(),
            Vec::new(),
            vec![GeneratedParam::Query {
                index: 1,
                key: "q".to_owned(),
            }],
        )],
    })
    .unwrap_err();
    assert!(error.message.contains("index 1 is out of bounds"));
}
