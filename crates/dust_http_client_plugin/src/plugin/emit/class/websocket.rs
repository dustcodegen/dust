use dust_dart_emit::{SerdeCodecDirection, render_serde_codec_constructor};
use dust_diagnostics::Diagnostic;
use dust_ir::TypeIr;

use super::*;
use crate::plugin::{
    emit::request::render_bound_path_expression, generated::GeneratedWsEndpoint,
    util::escape_single_quoted,
};

/// Renders connector wiring only for clients that contain socket methods.
pub(crate) fn render_ws_client_class(
    spec: &ClientSpec<'_>,
    runtime_prefix: &str,
    socket_methods: Vec<String>,
) -> String {
    /// Template values for the generated socket declaration.
    #[derive(Serialize)]
    struct Context<'a> {
        /// Public generated client class name.
        class_name: &'a str,
        /// Validated prefix of the typed WebSocket runtime import.
        runtime_prefix: &'a str,
        /// Rendered HTTP and WebSocket method declarations.
        methods: String,
    }
    let methods = spec
        .endpoints
        .iter()
        .map(|endpoint| render_endpoint_method(spec, endpoint, false))
        .chain(socket_methods)
        .collect::<Vec<_>>()
        .join("\n\n");
    render_template(
        "websocket_client_class",
        include_str!("../templates/websocket_client_class.jinja"),
        Context {
            class_name: spec.class_name,
            runtime_prefix,
            methods,
        },
    )
}

/// Renders a socket method without applying HTTP response classification.
pub(crate) fn render_ws_endpoint_method(
    endpoint: &GeneratedWsEndpoint,
    bindings: &[EndpointParam<'_>],
) -> Result<String, Diagnostic> {
    /// Template values for the generated socket declaration.
    #[derive(Serialize)]
    struct Context<'a> {
        /// Validated prefix of the typed WebSocket runtime import.
        runtime_prefix: &'a str,
        /// Client-to-server message type.
        send_type: String,
        /// Server-to-client message type.
        receive_type: String,
        /// Encoded route path expression.
        path: String,
        /// Statements binding handshake query values and headers.
        setup: String,
        /// Public serializer constructor for outgoing messages.
        serializer: String,
        /// Public deserializer constructor for incoming messages.
        deserializer: String,
        /// Rendered subprotocol list or null.
        protocols: String,
    }
    let codec = |ty: &TypeIr, direction| match ty {
        TypeIr::Named {
            name,
            args,
            nullable: false,
        } => Ok(render_serde_codec_constructor(name, args, direction)),
        _ => Err(Diagnostic::error(
            "generated WebSocket message types require non-nullable named Serde models",
        )),
    };
    let return_type = render_type(&endpoint.method.return_type);
    let params = render_method_parameters(
        &endpoint.method.params,
        &return_type,
        &endpoint.method.name,
        "async",
    );
    let body = render_template(
        "websocket_method_body",
        include_str!("../templates/websocket_method_body.jinja"),
        Context {
            runtime_prefix: &endpoint.runtime_prefix,
            send_type: render_type(&endpoint.send_type),
            receive_type: render_type(&endpoint.receive_type),
            path: render_bound_path_expression(&endpoint.path, bindings),
            setup: render_binding_setup(bindings),
            serializer: codec(&endpoint.send_type, SerdeCodecDirection::Serialize)?,
            deserializer: codec(&endpoint.receive_type, SerdeCodecDirection::Deserialize)?,
            protocols: match &endpoint.protocols {
                None => "null".to_owned(),
                Some(values) => format!(
                    "<String>[{}]",
                    values
                        .iter()
                        .map(|value| format!("'{}'", escape_single_quoted(value)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            },
        },
    );
    Ok(render_template(
        "endpoint_method",
        include_str!("../templates/endpoint_method.jinja"),
        EndpointMethodContext {
            doc_comment: format!("/// Connects to `WS {}`.\n  ", endpoint.path),
            override_annotation: "",
            return_type,
            method_name: &endpoint.method.name,
            params,
            async_marker: "async",
            body,
        },
    ))
}
