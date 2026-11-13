use dust_ir::{
    ConditionalUriIr, ExprSourceIr, FunctionIr, MethodIr, MethodParamIr, NameIr,
    NamespaceDirectiveIr, ParamKind, SpanIr,
};
use dust_text::{FileId, TextRange};

use super::*;

/// Regression tests for generated member and namespace collisions.
mod collisions;

/// Provides a stable source location for test declarations.
fn span() -> SpanIr {
    SpanIr::new(FileId::new(1), TextRange::new(0_u32, 1_u32))
}
/// Builds a plain prefixed package import for model reachability tests.
fn import(uri: &str, prefix: &str) -> ImportIr {
    ImportIr {
        namespace: None,
        uri: uri.to_owned(),
        prefix: Some(prefix.to_owned()),
        show: Vec::new(),
        hide: Vec::new(),
        is_deferred: false,
        span: span(),
    }
}
/// Builds a minimal server library for typed import selection.
fn file() -> DartFileIr {
    let mut file = DartFileIr::empty(".", "server", "lib/api.dart", "lib/api.g.dart", span());
    file.import_directives
        .push(import("package:models/models.dart", "models"));
    file
}
/// Builds a typed WebSocket fixture with distinct message directions.
fn socket() -> GeneratedWsEndpoint {
    GeneratedWsEndpoint {
        method: MethodIr {
            name: "chat".to_owned(),
            is_static: false,
            is_external: false,
            return_type: TypeIr::generic(
                "Future",
                vec![TypeIr::named("dustWs.WebSocketConnection")],
            ),
            has_body: false,
            body_source: None,
            params: Vec::new(),
            span: span(),
            traits: Vec::new(),
            configs: Vec::new(),
        },
        path: "/chat".to_owned(),
        params: Vec::new(),
        send_type: TypeIr::named("models.Command"),
        receive_type: TypeIr::generic("models.Event", vec![TypeIr::string()]),
        runtime_prefix: "dustWs".to_owned(),
        protocols: None,
    }
}
/// Builds a required positional handshake parameter.
fn param(name: &str) -> MethodParamIr {
    MethodParamIr {
        name: name.to_owned(),
        ty: TypeIr::string(),
        span: span(),
        kind: ParamKind::Named,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs: Vec::new(),
    }
}

/// Checks imports come from real payload and handshake types.
#[test]
fn socket_imports_collect_payloads_and_handshakes_without_synthetic_runtime_types() {
    let mut file = file();
    file.import_directives
        .push(import("package:query/query.dart", "query"));
    let mut socket = socket();
    let mut query = param("filter");
    query.ty = TypeIr::named("query.Filter");
    socket.method.params.push(query);
    let mut errors = Vec::new();
    assert_eq!(
        required_imports_with_ws(&file, "ApiClient", &[], &[socket], &mut errors),
        [
            "import 'package:models/models.dart' as models;",
            "import 'package:query/query.dart' as query;"
        ]
    );
    assert!(errors.is_empty());
}

/// Checks that source names cannot capture the chosen runtime prefix.
#[test]
fn runtime_prefix_avoids_imports_declarations_and_method_parameters() {
    let mut file = file();
    file.import_directives
        .push(import("package:other/other.dart", "dustWs"));
    file.functions.push(FunctionIr {
        name: NameIr {
            source: "dustWs2".to_owned(),
            short: "dustWs2".to_owned(),
            prefix: None,
            span: span(),
        },
        return_type: TypeIr::named("void"),
        params: vec![param("dustWs3")],
        annotations: Vec::new(),
        span: span(),
    });
    assert_eq!(runtime_prefix(&file, "dustWs4", []), "dustWs5");
}

/// Checks operation overrides reserve their final generated names.
#[test]
fn runtime_prefix_reserves_generated_operation_overrides() {
    assert_eq!(runtime_prefix(&file(), "ApiClient", []), "dustWs");
    assert_eq!(runtime_prefix(&file(), "ApiClient", ["dustWs"]), "dustWs2");
    assert_eq!(
        runtime_prefix(&file(), "ApiClient", ["dustWs2", "dustWs"]),
        "dustWs3"
    );
}

/// Rejects payload imports that a standalone client cannot safely expose.
#[test]
fn socket_imports_reject_private_unprefixed_and_unsafe_payload_imports() {
    for ty in [
        TypeIr::named("models._Secret"),
        TypeIr::named("Local"),
        TypeIr::function("void Function()"),
        TypeIr::named("missing.Model"),
    ] {
        let mut socket = socket();
        socket.send_type = ty;
        let mut errors = Vec::new();
        required_imports_with_ws(&file(), "ApiClient", &[], &[socket], &mut errors);
        assert!(!errors.is_empty());
    }
    for uri in [
        "../models.dart",
        "dart:io",
        "package:dust_server/server.dart",
    ] {
        let mut file = file();
        file.import_directives[0].uri = uri.to_owned();
        let mut errors = Vec::new();
        required_imports_with_ws(&file, "ApiClient", &[], &[socket()], &mut errors);
        assert!(!errors.is_empty());
    }
}

/// Rejects conditional WebSocket DTO imports while preserving HTTP behavior.
#[test]
fn socket_imports_reject_conditional_dto_branches_without_changing_http_imports() {
    let mut file = file();
    let import_span = SpanIr::new(FileId::new(7), TextRange::new(10_u32, 90_u32));
    let expression = |source: &str| ExprSourceIr {
        source: source.to_owned(),
        span: import_span,
    };
    let import = &mut file.import_directives[0];
    import.uri = "package:models/native.dart".to_owned();
    import.span = import_span;
    import.namespace = Some(NamespaceDirectiveIr {
        default_uri: expression("'package:models/native.dart'"),
        conditional_uris: vec![ConditionalUriIr {
            condition: expression("(dart.library.js_interop)"),
            uri: expression("'package:models/web.dart'"),
            span: import_span,
        }],
        combinators: Vec::new(),
    });
    let mut errors = Vec::new();
    assert!(required_imports_with_ws(&file, "ApiClient", &[], &[socket()], &mut errors).is_empty());
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].message,
        "generated WebSocket client import `package:models/native.dart` cannot use conditional branches"
    );
    assert_eq!(
        errors[0].labels,
        vec![SourceLabel::new(
            import_span.file_id,
            import_span.range,
            "use one plain prefixed package import for shared WebSocket models"
        )]
    );

    let mut http_errors = Vec::new();
    assert_eq!(
        super::super::import_for_prefix(&file, "models", &mut http_errors),
        Some("import 'package:models/native.dart' as models;".to_owned())
    );
    assert!(http_errors.is_empty());
}

/// Checks operation and parameter names against generated client bindings.
#[test]
fn socket_names_cannot_capture_imports_runtime_members_or_method_locals() {
    for name in [
        "models",
        "_dio",
        "_uri",
        "_headers",
        "_queryParameters",
        "_webSocketConnector",
    ] {
        let mut socket = socket();
        socket.method.params.push(param(name));
        let mut errors = Vec::new();
        required_imports_with_ws(&file(), "ApiClient", &[], &[socket], &mut errors);
        assert!(!errors.is_empty(), "{name}");
    }
    for name in ["ApiClient", "_baseUrl", "_webSocketUri", "models"] {
        let mut socket = socket();
        socket.method.name = name.to_owned();
        let mut errors = Vec::new();
        required_imports_with_ws(&file(), "ApiClient", &[], &[socket], &mut errors);
        assert!(!errors.is_empty(), "{name}");
    }
    for prefix in ["String", "Dio", "RequestOptions", "ApiClient", "bad-name"] {
        let mut file = file();
        file.import_directives[0].prefix = Some(prefix.to_owned());
        let mut socket = socket();
        socket.send_type = TypeIr::named(format!("{prefix}.Command"));
        let mut errors = Vec::new();
        required_imports_with_ws(&file, "ApiClient", &[], &[socket], &mut errors);
        assert!(!errors.is_empty(), "{prefix}");
    }
}
