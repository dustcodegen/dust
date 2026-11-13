use dust_ir::{MethodParamIr, ParamKind, SpanIr};
use dust_text::{FileId, TextRange};

use super::super::*;
use crate::plugin::generated::{GeneratedEndpoint, GeneratedHttpVerb, GeneratedRequestMode};

/// Builds a minimal asynchronous method with a stable source span.
pub(super) fn method(name: &str) -> MethodIr {
    MethodIr {
        name: name.to_owned(),
        is_static: false,
        is_external: false,
        return_type: TypeIr::generic("Future", vec![TypeIr::string()]),
        has_body: false,
        body_source: None,
        params: Vec::new(),
        span: SpanIr::new(FileId::new(1), TextRange::new(0_u32, 1_u32)),
        traits: Vec::new(),
        configs: Vec::new(),
    }
}

/// Builds one required positional handshake parameter.
pub(super) fn param(name: &str, ty: TypeIr) -> MethodParamIr {
    MethodParamIr {
        name: name.to_owned(),
        ty,
        span: method("span").span,
        kind: ParamKind::Positional,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs: Vec::new(),
    }
}

/// Builds a typed socket endpoint with path, query and header bindings.
pub(super) fn socket() -> GeneratedWsEndpoint {
    let send_type = TypeIr::named("models.Command");
    let receive_type = TypeIr::generic("models.Event", vec![TypeIr::string()]);
    let mut method = method("chat");
    method.return_type = TypeIr::generic(
        "Future",
        vec![TypeIr::generic(
            "dustWs.WebSocketConnection",
            vec![send_type.clone(), receive_type.clone()],
        )],
    );
    method.params = vec![
        param("room", TypeIr::string()),
        param("page", TypeIr::int().nullable()),
        param("token", TypeIr::string()),
    ];
    GeneratedWsEndpoint {
        method,
        path: "/rooms/{room}".to_owned(),
        params: vec![
            GeneratedParam::Path {
                index: 0,
                key: "room".to_owned(),
            },
            GeneratedParam::Query {
                index: 1,
                key: "page".to_owned(),
            },
            GeneratedParam::Header {
                index: 2,
                key: "x-token".to_owned(),
            },
        ],
        send_type,
        receive_type,
        runtime_prefix: "dustWs".to_owned(),
        protocols: Some(vec!["chat.v1".to_owned()]),
    }
}

/// Builds an empty standalone client using shared model imports.
pub(super) fn client() -> GeneratedClient {
    GeneratedClient {
        class_name: "ChatClient".to_owned(),
        imports: vec!["import 'package:models/models.dart' as models;".to_owned()],
        endpoints: Vec::new(),
    }
}

/// Builds the HTTP endpoint used by mixed-client parity tests.
pub(super) fn http() -> GeneratedEndpoint {
    GeneratedEndpoint {
        method: method("health"),
        verb: GeneratedHttpVerb::Get,
        path: "/health".to_owned(),
        request_mode: GeneratedRequestMode::Standard,
        params: Vec::new(),
        serde_response_types: Vec::new(),
    }
}
