use super::*;
use crate::plan::WebSocketPlan;

/// Builds a typed session plan for directional capability checks.
fn websocket_plan<'a>(
    handler: &'a FunctionIr,
    send: &'a TypeIr,
    receive: &'a TypeIr,
) -> ServerPlan<'a> {
    let mut server = plan(handler, None);
    server.functions[0].websocket = Some(WebSocketPlan {
        socket_index: 0,
        server_send: send,
        server_receive: receive,
        protocols: None,
        allowed_origins: None,
    });
    server
}

/// Checks server codec needs before enabling the reverse client direction.
#[test]
fn validates_server_and_optional_client_directions_independently() {
    let handler = function(TypeIr::named("void"));
    let send = TypeIr::named("contracts.Sent");
    let receive = TypeIr::named("contracts.Received");
    let mut sent = fact("Sent");
    sent.deserializable = false;
    let mut received = fact("Received");
    received.serializable = false;
    let index = models(&[sent, received]);
    let mut server = websocket_plan(&handler, &send, &receive);
    assert!(hydrate_payload_models(&file(), &mut server, &index, false).is_empty());
    let errors = hydrate_payload_models(&file(), &mut server, &index, true);
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(Diagnostic::has_labels));
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("send model `contracts.Sent` must derive Deserialize()")
    }));
    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("receive model `contracts.Received` must derive Serialize()")
    }));

    let mut sent = fact("Sent");
    sent.serializable = false;
    let mut received = fact("Received");
    received.deserializable = false;
    assert_eq!(
        hydrate_payload_models(&file(), &mut server, &models(&[sent, received]), false).len(),
        2
    );
}

/// Rejects message shapes without named nonnullable object codecs.
#[test]
fn websocket_frames_require_named_nonnullable_serde_object_models() {
    let handler = function(TypeIr::named("void"));
    let receive = TypeIr::named("contracts.Received");
    let models = models(&[fact("Received")]);
    for send in [
        TypeIr::dynamic(),
        TypeIr::named("contracts.Received").nullable(),
        TypeIr::string(),
        TypeIr::named("Missing"),
    ] {
        let mut server = websocket_plan(&handler, &send, &receive);
        let errors = hydrate_payload_models(&file(), &mut server, &models, false);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].has_labels());
        assert!(errors[0].message.contains("WebSocket send"));
    }
    let send = TypeIr::named("contracts.Missing");
    let mut server = websocket_plan(&handler, &send, &receive);
    assert_eq!(
        hydrate_payload_models(&file(), &mut server, &models, false).len(),
        1
    );
}
