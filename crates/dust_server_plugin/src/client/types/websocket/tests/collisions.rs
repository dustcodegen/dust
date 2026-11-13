use super::*;

/// Checks that operations and parameters cannot capture the URI type.
#[test]
fn websocket_operation_and_parameter_cannot_shadow_the_uri_encoder() {
    let reference = SpanIr::new(FileId::new(8), TextRange::new(20_u32, 40_u32));
    for operation in [true, false] {
        let mut socket = socket();
        let (message, label) = if operation {
            socket.method.name = "Uri".to_owned();
            socket.method.span = reference;
            (
                "generated client operation `Uri` conflicts with a generated client member",
                "rename the client operation",
            )
        } else {
            let mut argument = param("Uri");
            argument.span = reference;
            socket.method.params.push(argument);
            (
                "generated client parameter `Uri` conflicts with generated WebSocket setup",
                "rename the handshake parameter",
            )
        };
        let mut errors = Vec::new();
        required_imports_with_ws(&file(), "ApiClient", &[], &[socket], &mut errors);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, message);
        assert_eq!(
            errors[0].labels,
            vec![SourceLabel::new(reference.file_id, reference.range, label)]
        );
    }
}
