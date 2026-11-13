use super::*;

/// Renders a client with optional HTTP and WebSocket operation overrides.
fn render(http_operation: Option<&str>, socket_operation: &str) -> String {
    let send = TypeIr::named("models.Event");
    let receive = TypeIr::named("models.Command");
    let callables = [function("health"), function("chat")];
    let mut plan = empty_plan();
    let mut analysis = WorkspaceAnalysisBuilder::default();
    if let Some(operation) = http_operation {
        plan.functions.push(handler(&callables[0], operation));
        analysis.add_string_set_value(SERVER_ROUTES_ANALYSIS_KEY, fact("health", "GET", operation));
    }
    let mut socket = handler(&callables[1], socket_operation);
    socket.websocket = Some(WebSocketPlan {
        socket_index: 0,
        server_send: &send,
        server_receive: &receive,
        protocols: None,
        allowed_origins: None,
    });
    plan.functions.push(socket);
    analysis.add_string_set_value("dust_server.ws.v1", socket_fact("chat", socket_operation));
    generate_server_client(
        &source(),
        &plan,
        &destination(),
        &analysis.build(),
        &empty_payload_models(),
    )
    .unwrap()
    .remove(0)
    .1
}

/// Builds the expected output with the next available runtime prefix.
fn requalify(source: &str) -> String {
    source
        .replace(" as dustWs;", " as dustWs2;")
        .replace("dustWs.", "dustWs2.")
}

/// Checks a WebSocket operation override cannot capture the runtime import.
#[test]
fn websocket_operation_override_does_not_capture_the_runtime_prefix() {
    let baseline = render(None, "chat");
    let expected = requalify(&baseline).replace(" chat()", " dustWs()");
    assert_ne!(expected, baseline);
    assert_eq!(render(None, "dustWs"), expected);
}

/// Checks an HTTP override cannot capture a mixed client runtime import.
#[test]
fn mixed_http_operation_override_does_not_capture_the_runtime_prefix() {
    let baseline = render(Some("health"), "chat");
    let expected = requalify(&baseline).replace(" health()", " dustWs()");
    assert_ne!(expected, baseline);
    assert_eq!(render(Some("dustWs"), "chat"), expected);
}
