use super::*;

/// Regression tests for source names captured by generated adapters.
mod names;
/// Regression tests for explicit canonical socket identity proofs.
mod proof;

/// Builds a typed WebSocket fixture with distinct message directions.
fn socket(ty: TypeIr) -> MethodParamIr {
    let mut param = param("socket", "unused");
    param.ty = ty;
    param.configs.clear();
    param
}

/// Builds the direct generic Dust connection type used by planning tests.
fn socket_type() -> TypeIr {
    TypeIr::generic(
        "socket.WebSocketConnection",
        vec![TypeIr::named("Notice"), TypeIr::named("Command")],
    )
}

/// Builds a controller method with one typed WebSocket route.
fn chat() -> MethodIr {
    let mut chat = method("chat", vec![config("dust_server::WS", Some("/chat"))]);
    chat.return_type = TypeIr::generic("Future", vec![TypeIr::named("void")]);
    chat.params = vec![socket(socket_type())];
    chat
}

/// Wraps a handler in a controller-scoped Dart library fixture.
fn library(chat: MethodIr) -> DartFileIr {
    let mut library = file();
    library.classes = vec![class(vec![chat])];
    library
}

/// Checks a familiar type spelling cannot replace canonical identity proof.
#[test]
fn rejects_unproven_same_named_socket_identity() {
    assert!(
        messages(&library(chat()))
            .iter()
            .any(|message| message.contains("canonical Dust runtime class"))
    );
}

/// Checks the session parameter shape and exact connection count.
#[test]
fn requires_one_direct_nonnullable_unwrapped_typed_socket() {
    for params in [
        Vec::new(),
        vec![socket(socket_type()), socket(socket_type())],
        vec![socket(TypeIr::named("socket.WebSocketConnection"))],
        vec![socket(socket_type().nullable())],
        vec![socket(TypeIr::generic("Option", vec![socket_type()]))],
        vec![socket(TypeIr::generic(
            "Result",
            vec![socket_type(), TypeIr::named("Rejection")],
        ))],
    ] {
        let mut chat = chat();
        chat.params = params;
        assert!(
            messages(&library(chat))
                .iter()
                .any(|message| message.contains("WebSocket connection parameter")
                    || message.contains("exactly one WebSocketConnection"))
        );
    }
    let mut chat = chat();
    chat.params[0].configs = vec![config("dust_server::Body", None)];
    assert!(
        messages(&library(chat))
            .iter()
            .any(|message| message.contains("invalid WebSocket connection parameter"))
    );
}

/// Checks upgrade handlers reject HTTP payload returns and body readers.
#[test]
fn websocket_handlers_cannot_return_payloads_or_read_request_bodies() {
    for result in [
        TypeIr::string(),
        TypeIr::named("Unit"),
        TypeIr::generic("Future", vec![TypeIr::named("Notice")]),
        TypeIr::generic("Future", vec![TypeIr::named("void")]).nullable(),
    ] {
        let mut chat = chat();
        chat.return_type = result;
        assert!(
            messages(&library(chat))
                .iter()
                .any(|message| message.contains("must return void or Future<void>"))
        );
    }
    for extractor in [
        "dust_server::Body",
        "dust_server::RawBody",
        "dust_server::Field",
        "dust_server::Part",
    ] {
        let mut chat = chat();
        let mut body = param("body", extractor);
        body.ty = TypeIr::string();
        if matches!(extractor, "dust_server::Field" | "dust_server::Part") {
            body.configs[0].positional_args = vec![AnnotationValueIr::String("value".to_owned())];
        }
        chat.params.push(body);
        assert!(
            messages(&library(chat))
                .iter()
                .any(|message| message.contains("cannot read a request body"))
        );
    }
    for media in ["dust_server::FormUrlEncoded", "dust_server::MultiPart"] {
        let mut chat = chat();
        chat.configs.push(config(media, None));
        assert!(
            messages(&library(chat))
                .iter()
                .any(|message| message.contains("cannot read a request body"))
        );
    }
}

/// Checks structured handshake options and rejects HTTP status overrides.
#[test]
fn websocket_options_accept_only_structured_string_lists_and_no_status() {
    let mut chat = chat();
    chat.configs[0]
        .named_args
        .insert("protocols".to_owned(), AnnotationValueIr::Bool(true));
    chat.configs[0].named_args.insert(
        "allowedOrigins".to_owned(),
        AnnotationValueIr::List(vec![AnnotationValueIr::Null]),
    );
    chat.configs[0]
        .named_args
        .insert("status".to_owned(), AnnotationValueIr::Null);
    let diagnostics = messages(&library(chat));
    for expected in [
        "`protocols` must be a list of strings",
        "`allowedOrigins` must be a list of strings",
        "unknown server annotation argument `status`",
    ] {
        assert!(diagnostics.iter().any(|message| message == expected));
    }
}

/// Checks typed message directions and absent origin-list semantics.
#[test]
fn retains_typed_message_directions_and_optional_protocol_lists() {
    let mut chat = chat();
    chat.return_type = TypeIr::named("void");
    chat.configs[0].named_args.insert(
        "protocols".to_owned(),
        AnnotationValueIr::List(vec![AnnotationValueIr::String("chat.v1".to_owned())]),
    );
    chat.configs[0]
        .named_args
        .insert("allowedOrigins".to_owned(), AnnotationValueIr::Null);
    let library = library(chat);
    let method = &library.classes[0].methods[0];
    let mut diagnostics = Vec::new();
    let ws = super::super::websocket::parse_websocket(
        &library,
        Callable::Method(method),
        &Args::Config(&method.configs[0]),
        &|_| true,
        &mut diagnostics,
    )
    .unwrap();
    assert_eq!(ws.socket_index, 0);
    assert_eq!(ws.server_send.name(), Some("Notice"));
    assert_eq!(ws.server_receive.name(), Some("Command"));
    assert_eq!(ws.protocols, Some(vec!["chat.v1".to_owned()]));
    assert_eq!(ws.allowed_origins, None);
    assert!(diagnostics.is_empty());
}

/// Checks HTTP and upgrade handlers share route and operation namespaces.
#[test]
fn websocket_and_http_share_route_and_operation_collision_checks() {
    let mut chat = chat();
    let mut http = method("read", vec![config("dust_server::GET", Some("/chat"))]);
    let mut source = library(chat.clone());
    source.classes[0].methods.push(http.clone());
    assert!(
        messages(&source)
            .iter()
            .any(|message| message == "duplicate server route")
    );
    http.configs[0].positional_args = vec![AnnotationValueIr::String("/other".to_owned())];
    for handler in [&mut chat, &mut http] {
        handler.configs[0].named_args.insert(
            "operationId".to_owned(),
            AnnotationValueIr::String("shared".to_owned()),
        );
    }
    source.classes[0].methods = vec![chat, http];
    assert!(
        messages(&source)
            .iter()
            .any(|message| message.contains("duplicate server operation identifier"))
    );
}
