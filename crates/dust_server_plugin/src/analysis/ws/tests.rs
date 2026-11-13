use super::*;
use dust_ir::TypeIr;

/// Builds the shared route identity for a typed upgrade fact.
fn route() -> ServerRouteFact {
    ServerRouteFact {
        source_path: "lib/chat.dart".to_owned(),
        controller: Some("Chat".to_owned()),
        prefix: "/api".to_owned(),
        callable: "chat".to_owned(),
        method: "GET".to_owned(),
        path: "/chat".to_owned(),
        status: 101,
        operation_id: "chat".to_owned(),
        summary: None,
        description: None,
        tags: vec!["chat".to_owned()],
        deprecated: false,
        hidden: false,
        params: Vec::new(),
        return_type: "Future<void>".to_owned(),
    }
}

/// Checks separate WebSocket facts without changing the HTTP bucket.
#[test]
fn websocket_facts_preserve_server_directions_and_leave_http_bucket_empty() {
    let send = TypeIr::named("models.Notice");
    let receive = TypeIr::named("models.Command");
    let mut builder = WorkspaceAnalysisBuilder::default();
    add_fact(
        route(),
        &WebSocketPlan {
            socket_index: 0,
            server_send: &send,
            server_receive: &receive,
            protocols: Some(vec!["chat.v1".to_owned()]),
            allowed_origins: None,
        },
        &mut builder,
    );
    let analysis = builder.build();
    assert!(
        analysis
            .string_set(crate::analysis::SERVER_ROUTES_ANALYSIS_KEY)
            .is_none()
    );
    assert_eq!(analysis.string_set(SERVER_WS_ANALYSIS_KEY).unwrap(), &[r#"{"route":{"source_path":"lib/chat.dart","controller":"Chat","prefix":"/api","callable":"chat","method":"GET","path":"/chat","status":101,"operation_id":"chat","summary":null,"description":null,"tags":["chat"],"deprecated":false,"hidden":false,"params":[],"return_type":"Future<void>"},"server_send":"models.Notice","server_receive":"models.Command","protocols":["chat.v1"]}"#.to_owned()]);
    let facts = ws_facts(&analysis, "lib/chat.dart").unwrap();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].server_send, "models.Notice");
    assert_eq!(facts[0].server_receive, "models.Command");
    assert_eq!(
        facts[0].protocols.as_deref(),
        Some(["chat.v1".to_owned()].as_slice())
    );
    assert!(ws_facts(&analysis, "lib/other.dart").unwrap().is_empty());
}

/// Checks malformed facts become diagnostics instead of being ignored.
#[test]
fn malformed_websocket_facts_return_diagnostics() {
    let mut builder = WorkspaceAnalysisBuilder::default();
    builder.add_string_set_value(SERVER_WS_ANALYSIS_KEY, "{");
    assert!(ws_facts(&builder.build(), "lib/chat.dart").is_err());
    assert!(
        ws_facts(&WorkspaceAnalysis::default(), "lib/chat.dart")
            .unwrap()
            .is_empty()
    );
}
