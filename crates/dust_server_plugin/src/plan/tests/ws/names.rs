use super::*;
use dust_ir::ImportIr;

/// Checks local upgrade-name conflicts retain actionable source labels.
#[test]
fn raw_upgrade_name_shadows_are_source_labelled() {
    let mut source = library(chat());
    source.import_directives.push(ImportIr {
        namespace: None,
        uri: "package:dust_dart/ws.dart".to_owned(),
        prefix: Some("ws".to_owned()),
        show: Vec::new(),
        hide: Vec::new(),
        is_deferred: false,
        span: span(),
    });
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains("shadows the WebSocket upgrade function"))
    );
    source.import_directives.clear();
    source.classes[0].methods.push(method("ws", Vec::new()));
    let diagnostics = plan_server_with_type_origins(&source, &|param| param.name == "socket")
        .err()
        .unwrap();
    assert!(diagnostics.iter().all(Diagnostic::has_labels));
    assert!(diagnostics.iter().any(|error| {
        error
            .message
            .contains("shadows the WebSocket upgrade function")
    }));
    source.classes[0].methods.pop();
    source.functions.push(FunctionIr {
        name: NameIr {
            source: "ws".to_owned(),
            short: "ws".to_owned(),
            prefix: None,
            span: span(),
        },
        return_type: TypeIr::named("void"),
        params: Vec::new(),
        annotations: Vec::new(),
        span: span(),
    });
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains("shadows the WebSocket upgrade function"))
    );
}

/// Checks fixed adapter locals cannot capture source types or callables.
#[test]
fn fixed_adapter_locals_cannot_capture_socket_or_model_types() {
    for root in ["request", "_dustValue0", "_dustResult0"] {
        let mut chat = chat();
        chat.params[0].ty = TypeIr::generic(
            "socket.WebSocketConnection",
            vec![
                TypeIr::named(format!("{root}.Notice")),
                TypeIr::named("Command"),
            ],
        );
        let source = library(chat);
        assert!(
            messages(&source)
                .iter()
                .any(|message| message.contains("conflicts with a generated handler local"))
        );
    }
    let mut source = library(chat());
    source.classes[0].name = "request".to_owned();
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains("WebSocket controller name conflicts"))
    );
    let method = source.classes.remove(0).methods.remove(0);
    source.functions.push(FunctionIr {
        name: NameIr {
            source: "request".to_owned(),
            short: "request".to_owned(),
            prefix: None,
            span: span(),
        },
        return_type: method.return_type,
        params: method.params,
        annotations: vec![annotation("dust_server::WS", "/chat", [])],
        span: span(),
    });
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains("WebSocket callable name conflicts"))
    );
}
