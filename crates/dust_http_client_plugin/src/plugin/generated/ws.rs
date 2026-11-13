use dust_diagnostics::Diagnostic;
use dust_ir::{MethodIr, TypeIr};

use super::{GeneratedClient, GeneratedParam, bind_param, endpoint_spec, generate_client_library};
use crate::plugin::{
    emit::{
        render_combine_base_urls_helper, render_isolate_helpers, render_shared_helpers,
        render_ws_client_class, render_ws_endpoint_method,
    },
    model::{ClientSpec, HttpTargetMode},
};

/// Exact source and invalid-input regression tests.
#[cfg(test)]
mod tests;

/// One typed WebSocket route represented as a generated client method.
pub struct GeneratedWsEndpoint {
    /// Synthetic client method with normalized parameters and asynchronous socket result.
    pub method: MethodIr,
    /// Complete route path including group prefixes.
    pub path: String,
    /// Client-visible handshake bindings by method-parameter index.
    pub params: Vec<GeneratedParam>,
    /// Client-to-server model with a public generated serializer.
    pub send_type: TypeIr,
    /// Server-to-client model with a public generated deserializer.
    pub receive_type: TypeIr,
    /// Validated, collision-free prefix for the focused WebSocket runtime import.
    pub runtime_prefix: String,
    /// Server-declared subprotocols offered during the handshake.
    pub protocols: Option<Vec<String>>,
}

/// Renders a standalone client containing HTTP and typed WebSocket endpoints.
///
/// An empty socket slice preserves the existing HTTP-only output exactly.
pub fn generate_client_library_with_ws(
    client: &GeneratedClient,
    sockets: &[GeneratedWsEndpoint],
) -> Result<String, Diagnostic> {
    let Some(first) = sockets.first() else {
        return generate_client_library(client);
    };
    let endpoints = client
        .endpoints
        .iter()
        .map(endpoint_spec)
        .collect::<Result<Vec<_>, _>>()?;
    let spec = ClientSpec {
        class_name: &client.class_name,
        base_url: None,
        target: HttpTargetMode::Dart,
        generate_test: false,
        endpoints,
    };
    let mut methods = Vec::new();
    for endpoint in sockets {
        if endpoint.runtime_prefix != first.runtime_prefix {
            return Err(Diagnostic::error(
                "generated WebSocket endpoints must share one runtime prefix",
            ));
        }
        let params = endpoint
            .params
            .iter()
            .map(|binding| {
                if !matches!(
                    binding,
                    GeneratedParam::Path { .. }
                        | GeneratedParam::Query { .. }
                        | GeneratedParam::Queries { .. }
                        | GeneratedParam::SerdeQueryModel { .. }
                        | GeneratedParam::Header { .. }
                        | GeneratedParam::HeaderMap { .. }
                ) {
                    return Err(Diagnostic::error(
                        "generated WebSocket clients accept only path, query, and header bindings",
                    ));
                }
                bind_param(&endpoint.method.params, binding)
            })
            .collect::<Result<Vec<_>, _>>()?;
        methods.push(render_ws_endpoint_method(endpoint, &params)?);
    }
    let mut blocks = vec![
        dust_plugin_api::GENERATED_HEADER.to_owned(),
        "import 'package:dio/dio.dart';".to_owned(),
        format!(
            "import 'package:dust_dart/ws.dart' as {};",
            first.runtime_prefix
        ),
    ];
    blocks.extend(client.imports.iter().cloned());
    blocks.push(render_ws_client_class(
        &spec,
        &first.runtime_prefix,
        methods,
    ));
    blocks.extend(render_isolate_helpers(&spec));
    if !client.endpoints.is_empty() {
        blocks.extend(render_shared_helpers());
    } else {
        blocks.push(render_combine_base_urls_helper().to_owned());
    }
    blocks.push(
        include_str!("../emit/templates/websocket_uri.jinja")
            .trim_end()
            .to_owned(),
    );
    Ok(format!(
        "{}\n",
        blocks
            .iter()
            .map(|block| block.trim_end())
            .collect::<Vec<_>>()
            .join("\n\n")
    ))
}
