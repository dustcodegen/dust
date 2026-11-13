use super::*;

/// Checks canonical identity proof enables both supported callable kinds.
#[test]
fn explicit_identity_proof_enables_controller_and_top_level_plans() {
    let mut source = library(chat());
    assert!(plan_server(&source).is_err());
    let plan = plan_server_with_type_origins(&source, &|param| param.name == "socket")
        .unwrap()
        .unwrap();
    let handler = &plan.controllers[0].handlers[0];
    assert_eq!(handler.method, "GET");
    assert_eq!(handler.status, 101);
    assert!(matches!(
        handler.params[0].extraction,
        Extraction::WebSocket
    ));
    assert_eq!(
        handler.websocket.as_ref().unwrap().server_send.name(),
        Some("Notice")
    );
    assert_eq!(
        handler.websocket.as_ref().unwrap().server_receive.name(),
        Some("Command")
    );
    let method = source.classes.remove(0).methods.remove(0);
    source.functions.push(FunctionIr {
        name: NameIr {
            source: "chat".to_owned(),
            short: "chat".to_owned(),
            prefix: None,
            span: span(),
        },
        return_type: method.return_type,
        params: method.params,
        annotations: vec![annotation("dust_server::WS", "/chat", [])],
        span: span(),
    });
    let plan = plan_server_with_type_origins(&source, &|param| param.name == "socket")
        .unwrap()
        .unwrap();
    assert_eq!(plan.functions.len(), 1);
    assert!(plan.functions[0].websocket.is_some());
}
