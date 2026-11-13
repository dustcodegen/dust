use super::*;

/// Small normalized inputs shared by socket-generation tests.
mod support;
use support::*;

/// Pins complete WS-only and mixed client source contracts.
#[test]
fn websocket_only_and_mixed_clients_match_exact_snapshots() {
    let mut client = client();
    assert_eq!(
        generate_client_library_with_ws(&client, &[socket()]).unwrap(),
        include_str!("socket.dart.snapshot")
    );
    client.endpoints.push(http());
    assert_eq!(
        generate_client_library_with_ws(&client, &[socket()]).unwrap(),
        include_str!("mixed.dart.snapshot")
    );
}

/// Proves the additive entrypoint preserves existing HTTP output.
#[test]
fn empty_websocket_slice_preserves_existing_http_bytes() {
    let mut client = client();
    client.endpoints.push(http());
    assert_eq!(
        generate_client_library_with_ws(&client, &[]).unwrap(),
        generate_client_library(&client).unwrap()
    );
}

/// Rejects invalid socket bindings, models and runtime prefixes.
#[test]
fn socket_errors_reject_http_bindings_indexes_models_and_prefix_disagreement() {
    for binding in [
        GeneratedParam::Body { index: 0 },
        GeneratedParam::QueryModel { index: 0 },
        GeneratedParam::Field {
            index: 0,
            key: "x".to_owned(),
        },
    ] {
        let mut endpoint = socket();
        endpoint.params = vec![binding];
        assert!(
            generate_client_library_with_ws(&client(), &[endpoint])
                .unwrap_err()
                .message
                .contains("only path, query, and header")
        );
    }
    let mut endpoint = socket();
    endpoint.params = vec![GeneratedParam::Queries { index: 99 }];
    assert!(
        generate_client_library_with_ws(&client(), &[endpoint])
            .unwrap_err()
            .message
            .contains("out of bounds")
    );
    for invalid in [
        TypeIr::string(),
        TypeIr::named("models.Command").nullable(),
        TypeIr::dynamic(),
    ] {
        let mut endpoint = socket();
        endpoint.send_type = invalid.clone();
        assert!(generate_client_library_with_ws(&client(), &[endpoint]).is_err());
        let mut endpoint = socket();
        endpoint.receive_type = invalid;
        assert!(generate_client_library_with_ws(&client(), &[endpoint]).is_err());
    }
    let mut second = socket();
    second.runtime_prefix = "other".to_owned();
    assert!(
        generate_client_library_with_ws(&client(), &[socket(), second])
            .unwrap_err()
            .message
            .contains("one runtime prefix")
    );
}

/// Preserves optional handshake bindings and absent protocol configuration.
#[test]
fn socket_wire_maps_preserve_serde_query_null_guards_and_protocol_absence() {
    let mut endpoint = socket();
    endpoint.method.params = vec![
        param("query", TypeIr::named("models.Query").nullable()),
        param(
            "headers",
            TypeIr::map_of(TypeIr::string(), TypeIr::string()).nullable(),
        ),
        param(
            "queries",
            TypeIr::map_of(TypeIr::string(), TypeIr::string()),
        ),
    ];
    endpoint.params = vec![
        GeneratedParam::SerdeQueryModel { index: 0 },
        GeneratedParam::HeaderMap { index: 1 },
        GeneratedParam::Queries { index: 2 },
    ];
    endpoint.path = "/chat".to_owned();
    endpoint.protocols = None;
    let source = generate_client_library_with_ws(&client(), &[endpoint]).unwrap();
    assert!(source.contains("query.serialize()"));
    assert!(source.contains("if (headers != null)"));
    assert!(source.contains("_queryParameters.addAll(queries)"));
    assert!(source.contains("protocols: null"));
}
