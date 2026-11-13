use dust_diagnostics::Diagnostic;
use dust_ir::{AnnotationValueIr, DartFileIr, MethodParamIr, TypeIr};

use super::{
    Callable, Extraction, ExtractionWrapper, ParamPlan, body::BodyMedia, model::Args, values::error,
};

/// Names captured by the generated request adapter or raw upgrade call.
mod names;

/// Validated typed messages and handshake options for one WebSocket route.
pub(crate) struct WebSocketPlan<'a> {
    /// Socket position in the handwritten callable's complete parameter list.
    pub(crate) socket_index: usize,
    /// JSON object model sent by the server.
    pub(crate) server_send: &'a TypeIr,
    /// JSON object model received by the server.
    pub(crate) server_receive: &'a TypeIr,
    /// Optional subprotocol offers, preserving absent versus empty lists.
    pub(crate) protocols: Option<Vec<String>>,
    /// Optional origin allow-list, preserving absent versus empty lists.
    pub(crate) allowed_origins: Option<Vec<String>>,
}

/// Checks canonical annotation ownership, never its short spelling.
pub(super) fn is_websocket(args: &Args<'_>) -> bool {
    match args {
        Args::Config(value) => value.symbol.0 == "dust_server::WS",
        Args::Annotation(value) => value
            .resolved_symbol
            .as_ref()
            .is_some_and(|symbol| symbol.0 == "dust_server::WS"),
    }
}

/// Validates the socket shape before its owning runtime class is proven.
pub(super) fn parse_websocket<'a>(
    file: &DartFileIr,
    callable: Callable<'a>,
    args: &Args<'_>,
    has_type_origin: &dyn Fn(&MethodParamIr) -> bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<WebSocketPlan<'a>> {
    let result = callable.return_type();
    let result = if result.is_named("Future") && !result.is_nullable() && result.args().len() == 1 {
        &result.args()[0]
    } else {
        result
    };
    if !result.is_named("void") || result.is_nullable() || !result.args().is_empty() {
        diagnostics.push(error(
            callable.span(),
            "WebSocket handlers must return void or Future<void>",
            "handle the session inside the handler without returning an HTTP payload",
        ));
    }
    let candidates = callable
        .params()
        .iter()
        .enumerate()
        .filter(|(_, param)| {
            has_type_origin(param)
                || param.ty.name().and_then(|name| name.rsplit('.').next())
                    == Some("WebSocketConnection")
        })
        .collect::<Vec<_>>();
    if candidates.len() != 1 {
        diagnostics.push(error(
            callable.span(),
            "WebSocket handlers require exactly one WebSocketConnection<Send, Receive>",
            "use one direct, nonnullable and unwrapped typed Dust connection",
        ));
        return None;
    }
    let (socket_index, param) = candidates[0];
    if param.ty.is_nullable() || param.ty.args().len() != 2 || !param.configs.is_empty() {
        diagnostics.push(error(
            param.span,
            "invalid WebSocket connection parameter",
            "use one unannotated, nonnullable WebSocketConnection<Send, Receive>",
        ));
        return None;
    }
    if !has_type_origin(param) {
        diagnostics.push(error(
            param.span,
            "WebSocket connection must resolve to the canonical Dust runtime class",
            "import WebSocketConnection from dust_dart/ws.dart; socket typedef aliases are unsupported",
        ));
    }
    names::validate(file, callable, diagnostics);
    Some(WebSocketPlan {
        socket_index,
        server_send: &param.ty.args()[0],
        server_receive: &param.ty.args()[1],
        protocols: string_list(args, "protocols", diagnostics),
        allowed_origins: string_list(args, "allowedOrigins", diagnostics),
    })
}

/// Records the socket without attempting a request extractor before upgrade.
pub(super) fn socket_param(param: &MethodParamIr) -> ParamPlan<'_> {
    ParamPlan {
        param,
        value_type: &param.ty,
        wrapper: ExtractionWrapper::Required,
        extraction: Extraction::WebSocket,
        validated: false,
        serde_deserialized: false,
    }
}

/// Prevents request-body readers from consuming WebSocket handshake data.
pub(super) fn validate_handshake(
    callable: Callable<'_>,
    media: BodyMedia,
    params: &[ParamPlan<'_>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if media != BodyMedia::Json
        || params.iter().any(|param| {
            matches!(
                param.extraction,
                Extraction::Body(_)
                    | Extraction::RawBody(_)
                    | Extraction::Field(_)
                    | Extraction::Part { .. }
            )
        })
    {
        diagnostics.push(error(
            callable.span(),
            "WebSocket handshakes cannot read a request body",
            "use path, query, header, state or request-parts extractors before upgrading",
        ));
    }
}

/// Reads only structured string lists accepted by the runtime handshake API.
fn string_list(
    args: &Args<'_>,
    key: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<String>> {
    match args.named().get(key)? {
        AnnotationValueIr::Null => None,
        AnnotationValueIr::List(values)
            if values
                .iter()
                .all(|value| matches!(value, AnnotationValueIr::String(_))) =>
        {
            Some(
                values
                    .iter()
                    .filter_map(|value| match value {
                        AnnotationValueIr::String(value) => Some(value.clone()),
                        _ => None,
                    })
                    .collect(),
            )
        }
        _ => {
            diagnostics.push(error(
                args.span(),
                format!("`{key}` must be a list of strings"),
                "use a const string list",
            ));
            None
        }
    }
}
