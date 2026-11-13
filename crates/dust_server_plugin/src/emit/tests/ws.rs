use super::*;
use crate::plan::WebSocketPlan;

/// Pins the complete adapter with extraction before upgrade and stable indices.
#[test]
fn typed_upgrade_extracts_before_callback_and_preserves_parameter_indices() {
    let mut function = function();
    function.name.source = "chat".to_owned();
    function.name.short = "chat".to_owned();
    function.return_type = TypeIr::generic("Future", vec![TypeIr::named("void")]);
    function.params = vec![
        param("room", TypeIr::string(), ParamKind::Positional),
        param(
            "socket",
            TypeIr::generic(
                "socket.WebSocketConnection",
                vec![
                    TypeIr::generic("models.Notice", vec![TypeIr::int()]),
                    TypeIr::named("Command"),
                ],
            ),
            ParamKind::Named,
        ),
        param("token", TypeIr::string(), ParamKind::Named),
    ];
    let socket = &function.params[1].ty;
    let handler = HandlerPlan {
        callable: Callable::Function(&function),
        method: "GET",
        path: "/chat/{room}".to_owned(),
        status: 101,
        params: function
            .params
            .iter()
            .zip([
                Extraction::Path("room".to_owned()),
                Extraction::WebSocket,
                Extraction::Header("authorization".to_owned()),
            ])
            .map(|(param, extraction)| ParamPlan {
                param,
                value_type: &param.ty,
                wrapper: ExtractionWrapper::Required,
                extraction,
                validated: false,
                serde_deserialized: false,
            })
            .collect(),
        middleware: Vec::new(),
        metadata: metadata("chat"),
        websocket: Some(WebSocketPlan {
            socket_index: 1,
            server_send: &socket.args()[0],
            server_receive: &socket.args()[1],
            protocols: Some(vec!["chat.v1".to_owned()]),
            allowed_origins: Some(vec!["https://chat.example".to_owned()]),
        }),
    };
    assert_eq!(
        emit_handler(&handler, None),
        include_str!("ws_adapter.dart.snapshot").trim_end()
    );
}

/// Checks callback parameters cannot capture source callables or types.
#[test]
fn callback_names_do_not_capture_top_level_callables_or_controller_types() {
    let mut function = function();
    function.name.source = "_dustSession".to_owned();
    function.params = vec![param(
        "socket",
        TypeIr::generic(
            "WebSocketConnection",
            vec![TypeIr::named("Notice"), TypeIr::named("Command")],
        ),
        ParamKind::Positional,
    )];
    let socket = &function.params[0].ty;
    let handler = HandlerPlan {
        callable: Callable::Function(&function),
        method: "GET",
        path: "/chat".to_owned(),
        status: 101,
        params: vec![ParamPlan {
            param: &function.params[0],
            value_type: socket,
            wrapper: ExtractionWrapper::Required,
            extraction: Extraction::WebSocket,
            validated: false,
            serde_deserialized: false,
        }],
        middleware: Vec::new(),
        metadata: metadata("chat"),
        websocket: Some(WebSocketPlan {
            socket_index: 0,
            server_send: &socket.args()[0],
            server_receive: &socket.args()[1],
            protocols: None,
            allowed_origins: None,
        }),
    };
    let output = emit_handler(&handler, None);
    assert!(output.contains("(_dustSession2) async {"));
    assert!(output.contains("await _dustSession(_dustValue0);"));
    assert!(
        emit_handler(&handler, Some("_dustSession"))
            .contains("await (this as _dustSession)._dustSession(_dustValue0);")
    );
}
