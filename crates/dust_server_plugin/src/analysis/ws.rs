use dust_diagnostics::Diagnostic;
use dust_plugin_api::{WorkspaceAnalysis, WorkspaceAnalysisBuilder};
use serde::{Deserialize, Serialize};

use super::{ServerRouteFact, render_type};
use crate::plan::WebSocketPlan;

/// Versioned facts for typed upgrades; HTTP route consumers remain unchanged.
const SERVER_WS_ANALYSIS_KEY: &str = "dust_server.ws.v1";

/// Canonical route identity and server-oriented message directions.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ServerWsFact {
    /// Shared callable, bindings, grouping and documentation facts.
    pub(crate) route: ServerRouteFact,
    /// Rendered model the server sends; consumers correlate the typed plan.
    pub(crate) server_send: String,
    /// Rendered model the server receives; consumers correlate the typed plan.
    pub(crate) server_receive: String,
    /// Protocols offered by generated clients and the server.
    pub(crate) protocols: Option<Vec<String>>,
}

/// Records a typed upgrade without adding a route to the HTTP fact bucket.
pub(super) fn add_fact(
    route: ServerRouteFact,
    websocket: &WebSocketPlan<'_>,
    analysis: &mut WorkspaceAnalysisBuilder,
) {
    let fact = ServerWsFact {
        route,
        server_send: render_type(websocket.server_send),
        server_receive: render_type(websocket.server_receive),
        protocols: websocket.protocols.clone(),
    };
    if let Ok(value) = serde_json::to_string(&fact) {
        analysis.add_string_set_value(SERVER_WS_ANALYSIS_KEY, value);
    }
}

/// Reads this library's typed upgrades from the same workspace analysis pass.
pub(crate) fn ws_facts(
    analysis: &WorkspaceAnalysis,
    source_path: &str,
) -> Result<Vec<ServerWsFact>, Diagnostic> {
    analysis
        .string_set(SERVER_WS_ANALYSIS_KEY)
        .unwrap_or_default()
        .iter()
        .map(|value| {
            serde_json::from_str::<ServerWsFact>(value).map_err(|error| {
                Diagnostic::error(format!(
                    "invalid {SERVER_WS_ANALYSIS_KEY} route fact: {error}"
                ))
            })
        })
        .filter_map(|result| match result {
            Ok(fact) if fact.route.source_path == source_path => Some(Ok(fact)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

/// Focused WebSocket regression tests.
#[cfg(test)]
mod tests;
