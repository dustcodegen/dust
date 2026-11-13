use dust_diagnostics::Diagnostic;
use dust_ir::{DartFileIr, SpanIr};

use super::{PayloadCapabilityIndex, label_payload_error, payload_capabilities, payload_error};
use crate::plan::WebSocketPlan;

/// Validates both server message directions and optional reverse client needs.
pub(super) fn validate_messages(
    file: &DartFileIr,
    websocket: &WebSocketPlan<'_>,
    span: SpanIr,
    models: &PayloadCapabilityIndex,
    require_client: bool,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (ty, direction, serialize, deserialize) in [
        (websocket.server_send, "send", true, require_client),
        (websocket.server_receive, "receive", require_client, true),
    ] {
        let Some(name) = ty.name().filter(|_| !ty.is_nullable()) else {
            diagnostics.push(payload_error(
                span,
                format!(
                    "WebSocket {direction} messages require a nonnullable named JSON-object model"
                ),
            ));
            continue;
        };
        match payload_capabilities(file, models, name) {
            Ok(Some(capabilities)) => {
                if serialize && !capabilities.serializable {
                    diagnostics.push(payload_error(span, format!(
                        "WebSocket {direction} model `{name}` must derive Serialize()"
                    )));
                }
                if deserialize && !capabilities.deserializable {
                    diagnostics.push(payload_error(span, format!(
                        "WebSocket {direction} model `{name}` must derive Deserialize()"
                    )));
                }
            }
            Ok(None) => diagnostics.push(payload_error(span, format!(
                "WebSocket {direction} model `{name}` must be a named JSON-object model declared locally or imported with a prefix and derive Dust Serde contracts"
            ))),
            Err(error) => diagnostics.push(label_payload_error(error, span)),
        }
    }
    diagnostics
}
