use dust_http_client_plugin::GeneratedWsEndpoint;

use super::*;
use crate::analysis::ServerWsFact;

/// Focused WebSocket regression tests.
#[cfg(test)]
mod tests;

/// Reverses server socket directions once while retaining handshake bindings.
pub(crate) fn build_ws_endpoint(
    fact: &ServerWsFact,
    handler: &HandlerPlan<'_>,
    runtime_prefix: &str,
) -> Result<GeneratedWsEndpoint, Diagnostic> {
    let websocket = handler.websocket.as_ref().ok_or_else(|| {
        Diagnostic::error(format!(
            "WebSocket fact `{}` no longer matches a typed socket handler",
            fact.route.operation_id
        ))
    })?;
    let mut params = Vec::new();
    let mut bindings = Vec::new();
    for fact_param in &fact.route.params {
        if fact_param.binding == "websocket" {
            continue;
        }
        let plan = handler
            .params
            .iter()
            .find(|value| value.param.name == fact_param.name)
            .ok_or_else(|| {
                Diagnostic::error(format!(
                    "route fact parameter `{}` no longer matches `{}`",
                    fact_param.name, fact.route.operation_id
                ))
            })?;
        if let Some(binding) = binding(fact_param, plan, params.len())? {
            params.push(client_param(plan));
            bindings.push(binding);
        }
    }
    let send_type = websocket.server_receive.clone();
    let receive_type = websocket.server_send.clone();
    let result = TypeIr::generic(
        format!("{runtime_prefix}.WebSocketConnection"),
        vec![send_type.clone(), receive_type.clone()],
    );
    Ok(GeneratedWsEndpoint {
        method: MethodIr {
            name: fact.route.operation_id.clone(),
            is_static: false,
            is_external: false,
            return_type: TypeIr::generic("Future", vec![result]),
            has_body: false,
            body_source: None,
            params,
            span: handler.callable.span(),
            traits: Vec::new(),
            configs: Vec::new(),
        },
        path: join_path(&fact.route.prefix, &fact.route.path),
        params: bindings,
        send_type,
        receive_type,
        runtime_prefix: runtime_prefix.to_owned(),
        protocols: websocket.protocols.clone(),
    })
}
