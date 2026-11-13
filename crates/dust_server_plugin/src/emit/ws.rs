use dust_dart_emit::{
    DYNAMIC_TYPES, NameAllocator, SerdeCodecDirection, dart_string_literal,
    render_serde_codec_constructor,
};
use dust_ir::TypeIr;

use super::push_call;
use crate::plan::{HandlerPlan, WebSocketPlan};

/// Returns the raw runtime upgrade response and invokes user code after upgrade.
pub(super) fn push_upgrade(
    lines: &mut Vec<String>,
    handler: &HandlerPlan<'_>,
    websocket: &WebSocketPlan<'_>,
    target: &str,
    arguments: &[String],
    controller: Option<&str>,
) {
    let socket = &handler.params[websocket.socket_index];
    let mut reserved = Vec::new();
    collect_type_names(&socket.param.ty, &mut reserved);
    reserved.push(handler.callable.name().to_owned());
    reserved.extend(controller.map(str::to_owned));
    let session = NameAllocator::new(reserved).allocate("_dustSession");
    let ty = DYNAMIC_TYPES.render(&socket.param.ty);
    lines.push("  return ws(".to_owned());
    lines.push(format!("    ({session}) async {{"));
    lines.push(format!(
        "      final _dustValue{} =",
        websocket.socket_index
    ));
    lines.push(format!("          {ty}.fromChannel("));
    lines.push(format!("            {session}.channel,"));
    lines.push(format!(
        "            serializer: {},",
        render_serde_codec_constructor(
            websocket.server_send.name().unwrap_or_default(),
            websocket.server_send.args(),
            SerdeCodecDirection::Serialize
        )
    ));
    lines.push(format!(
        "            deserializer: {},",
        render_serde_codec_constructor(
            websocket.server_receive.name().unwrap_or_default(),
            websocket.server_receive.args(),
            SerdeCodecDirection::Deserialize
        )
    ));
    lines.push(format!(
        "            negotiatedProtocol: {session}.protocol,"
    ));
    lines.push("          );".to_owned());
    push_call(
        lines,
        "      ",
        target,
        handler.callable.name(),
        arguments,
        ";",
    );
    lines.push("    },".to_owned());
    for (key, value) in [
        ("protocols", &websocket.protocols),
        ("allowedOrigins", &websocket.allowed_origins),
    ] {
        if let Some(values) = value {
            let values = values
                .iter()
                .map(|value| dart_string_literal(value))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("    {key}: const [{values}],"));
        }
    }
    lines.push("  ).handlers['GET']!(request);".to_owned());
}

/// Keeps callback names from shadowing written type names or import prefixes.
fn collect_type_names(ty: &TypeIr, names: &mut Vec<String>) {
    if let Some(name) = ty.name() {
        names.push(name.split('.').next().unwrap_or(name).to_owned());
    }
    for argument in ty.args() {
        collect_type_names(argument, names);
    }
}

/// Focused WebSocket regression tests.
#[cfg(test)]
mod tests {
    use super::*;

    /// Checks callback names avoid nested model prefixes and local type names.
    #[test]
    fn avoids_shadowing_nested_message_type_prefixes() {
        let mut prefixes = Vec::new();
        collect_type_names(
            &TypeIr::generic(
                "socket.WebSocketConnection",
                vec![
                    TypeIr::generic("models.Notice", vec![TypeIr::named("_dustSession.Id")]),
                    TypeIr::named("Command"),
                ],
            ),
            &mut prefixes,
        );
        assert_eq!(
            NameAllocator::new(prefixes).allocate("_dustSession"),
            "_dustSession2"
        );
        let mut names = Vec::new();
        collect_type_names(&TypeIr::named("_dustSession"), &mut names);
        assert_eq!(
            NameAllocator::new(names).allocate("_dustSession"),
            "_dustSession2"
        );
    }
}
