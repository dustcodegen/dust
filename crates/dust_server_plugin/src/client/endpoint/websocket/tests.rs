use super::*;
use crate::{
    client::endpoint::tests::{metadata, param, param_fact, route_fact},
    plan::{Extraction, WebSocketPlan},
};

/// Checks one direction reversal and stable handshake parameter indices.
#[test]
fn conversion_reverses_socket_directions_once_and_preserves_handshake_indices() {
    let send = TypeIr::generic("models.Event", vec![TypeIr::string()]);
    let receive = TypeIr::named("models.Command");
    let socket_type = TypeIr::generic(
        "ws.WebSocketConnection",
        vec![send.clone(), receive.clone()],
    );
    let method = MethodIr {
        name: "chat".to_owned(),
        is_static: false,
        is_external: false,
        return_type: TypeIr::named("void"),
        has_body: false,
        body_source: None,
        params: vec![
            param("room", TypeIr::string()),
            param("socket", socket_type),
            param("token", TypeIr::string()),
        ],
        span: crate::client::endpoint::tests::span(),
        traits: Vec::new(),
        configs: Vec::new(),
    };
    let mut handler = HandlerPlan {
        callable: Callable::Method(&method),
        method: "GET",
        path: "/chat/{room}".to_owned(),
        status: 200,
        params: method
            .params
            .iter()
            .enumerate()
            .map(|(index, param)| ParamPlan {
                param,
                value_type: &param.ty,
                wrapper: ExtractionWrapper::Required,
                extraction: if index == 1 {
                    Extraction::WebSocket
                } else {
                    Extraction::State
                },
                validated: false,
                serde_deserialized: false,
            })
            .collect(),
        middleware: Vec::new(),
        metadata: metadata(),
        websocket: Some(WebSocketPlan {
            socket_index: 1,
            server_send: &send,
            server_receive: &receive,
            protocols: Some(vec!["chat.v1".to_owned()]),
            allowed_origins: None,
        }),
    };
    let mut route = route_fact(vec![
        param_fact("room", "path"),
        param_fact("socket", "websocket"),
        param_fact("token", "header"),
    ]);
    route.path = "/chat/{room}".to_owned();
    let mut fact = ServerWsFact {
        route,
        server_send: "models.Event<String>".to_owned(),
        server_receive: "models.Command".to_owned(),
        protocols: handler.websocket.as_ref().unwrap().protocols.clone(),
    };
    let endpoint = build_ws_endpoint(&fact, &handler, "dustWs").unwrap();
    assert_eq!(endpoint.send_type, receive);
    assert_eq!(endpoint.receive_type, send);
    assert_eq!(
        endpoint.method.return_type,
        TypeIr::generic(
            "Future",
            vec![TypeIr::generic(
                "dustWs.WebSocketConnection",
                vec![receive.clone(), send.clone()]
            )]
        )
    );
    assert_eq!(endpoint.path, "/api/chat/{room}");
    assert_eq!(
        endpoint
            .method
            .params
            .iter()
            .map(|param| param.name.as_str())
            .collect::<Vec<_>>(),
        ["room", "token"]
    );
    assert!(matches!(
        &endpoint.params[..],
        [
            GeneratedParam::Path { index: 0, .. },
            GeneratedParam::Header { index: 1, .. }
        ]
    ));
    assert_eq!(endpoint.protocols, Some(vec!["chat.v1".to_owned()]));
    fact.route.params.push(param_fact("missing", "query"));
    assert!(build_ws_endpoint(&fact, &handler, "dustWs").is_err());
    handler.websocket = None;
    assert!(
        build_ws_endpoint(&fact, &handler, "dustWs")
            .err()
            .unwrap()
            .message
            .contains("typed socket handler")
    );
}
