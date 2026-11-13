use dust_ir::ImportIr;

use super::*;
use crate::plan::WebSocketPlan;

/// Generated operation names must not capture the runtime import.
mod prefix;

/// Serializes one WebSocket fact with server-oriented model names.
fn socket_fact(callable: &str, operation: &str) -> String {
    json!({
        "route": serde_json::from_str::<serde_json::Value>(&fact(callable, "GET", operation)).unwrap(),
        "server_send": "models.Event", "server_receive": "models.Command", "protocols": null,
    }).to_string()
}

/// Builds a server source fixture with shared message contracts.
fn source() -> DartFileIr {
    let mut file = file("/server/lib/api.dart");
    file.import_directives = vec![ImportIr {
        namespace: None,
        uri: "package:models/models.dart".to_owned(),
        prefix: Some("models".to_owned()),
        show: Vec::new(),
        hide: Vec::new(),
        is_deferred: false,
        span: file.span,
    }];
    file
}

/// Checks WebSocket-only output generation, claims, and tag filtering.
#[test]
fn websocket_only_groups_emit_clients_and_claim_the_same_outputs() {
    let send = TypeIr::named("models.Event");
    let receive = TypeIr::named("models.Command");
    let callable = function("chat");
    let mut websocket = handler(&callable, "chat");
    websocket.websocket = Some(WebSocketPlan {
        socket_index: 0,
        server_send: &send,
        server_receive: &receive,
        protocols: None,
        allowed_origins: None,
    });
    let mut plan = empty_plan();
    plan.functions.push(websocket);
    let file = source();
    let mut analysis = WorkspaceAnalysisBuilder::default();
    analysis.add_string_set_value("dust_server.ws.v1", socket_fact("chat", "chat"));
    collect_output_claims(&file, &plan, &[destination()], &mut analysis);
    let analysis = analysis.build();
    let outputs = generate_server_client(
        &file,
        &plan,
        &destination(),
        &analysis,
        &empty_payload_models(),
    )
    .unwrap();
    assert_eq!(outputs.len(), 2);
    assert_eq!(
        outputs[0].0,
        PathBuf::from("/client/lib/src/generated/api_client.routes.g.dart")
    );
    assert!(is_client_output_claimed(&analysis, &outputs[0].0));
    assert!(is_client_output_claimed(&analysis, &outputs[1].0));
    assert!(
        outputs[0]
            .1
            .contains("Future<dustWs.WebSocketConnection<models.Command, models.Event>>")
    );
    assert_eq!(
        outputs[1].1,
        format!("{GENERATED_HEADER}\nexport 'api_client.routes.g.dart';\n")
    );
    let mut filtered = destination();
    filtered.tags.push("excluded".to_owned());
    assert!(
        generate_server_client(&file, &plan, &filtered, &analysis, &empty_payload_models())
            .unwrap()
            .is_empty()
    );
}

/// Checks mixed output groups and inconsistent operation facts.
#[test]
fn mixed_groups_share_clients_and_detect_duplicate_operations_and_missing_handlers() {
    let send = TypeIr::named("models.Event");
    let receive = TypeIr::named("models.Command");
    let callables = [function("health"), function("chat")];
    let mut plan = empty_plan();
    plan.functions.push(handler(&callables[0], "health"));
    let mut socket = handler(&callables[1], "chat");
    socket.websocket = Some(WebSocketPlan {
        socket_index: 0,
        server_send: &send,
        server_receive: &receive,
        protocols: Some(vec![]),
        allowed_origins: None,
    });
    plan.functions.push(socket);
    let file = source();
    let mut analysis = WorkspaceAnalysisBuilder::default();
    analysis.add_string_set_value(SERVER_ROUTES_ANALYSIS_KEY, fact("health", "GET", "health"));
    analysis.add_string_set_value("dust_server.ws.v1", socket_fact("chat", "chat"));
    let outputs = generate_server_client(
        &file,
        &plan,
        &destination(),
        &analysis.clone().build(),
        &empty_payload_models(),
    )
    .unwrap();
    assert_eq!(outputs.len(), 2);
    assert!(outputs[0].1.contains("health()"));
    assert!(outputs[0].1.contains("chat()"));
    assert!(outputs[0].1.contains("protocols: <String>[]"));
    analysis.add_string_set_value("dust_server.ws.v1", socket_fact("chat", "health"));
    analysis.add_string_set_value("dust_server.ws.v1", socket_fact("missing", "absent"));
    let errors = generate_server_client(
        &file,
        &plan,
        &destination(),
        &analysis.build(),
        &empty_payload_models(),
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("more than once"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("no longer matches"))
    );
}

/// Checks malformed WebSocket facts when no HTTP routes are present.
#[test]
fn malformed_websocket_facts_are_diagnostics_even_without_http_routes() {
    let mut analysis = WorkspaceAnalysisBuilder::default();
    analysis.add_string_set_value("dust_server.ws.v1", "{");
    let errors = generate_server_client(
        &source(),
        &empty_plan(),
        &destination(),
        &analysis.build(),
        &empty_payload_models(),
    )
    .unwrap_err();
    assert!(errors[0].message.contains("invalid dust_server.ws.v1"));
}
