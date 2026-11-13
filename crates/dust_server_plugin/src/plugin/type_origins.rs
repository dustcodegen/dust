use dust_ir::{DartFileIr, MethodParamIr, TypeIr};
use dust_plugin_api::TypeOriginRequest;

/// Owning declaration of the concrete typed WebSocket runtime class.
pub(crate) const CONNECTION_LIBRARY: &str = "package:dust_dart/src/ws/connection.dart";
/// Concrete class name; typedefs cannot preserve message argument ordering yet.
pub(crate) const CONNECTION_NAME: &str = "WebSocketConnection";

/// Requests only root named types, without interpreting generic aliases.
pub(crate) fn type_origin_request(param: &MethodParamIr) -> Option<TypeOriginRequest> {
    match &param.ty {
        TypeIr::Named { name, .. } => Some(TypeOriginRequest {
            name: name.to_string(),
            span: param.span,
        }),
        _ => None,
    }
}

/// Limits declaration-origin work to parameters of actual WS callables.
pub(super) fn requests(file: &DartFileIr) -> Vec<TypeOriginRequest> {
    let methods = file
        .classes
        .iter()
        .filter(|class| {
            class
                .configs
                .iter()
                .any(|config| config.symbol.0 == "dust_server::Controller")
        })
        .flat_map(|class| class.methods.iter())
        .filter(|method| {
            method
                .configs
                .iter()
                .any(|config| config.symbol.0 == "dust_server::WS")
        })
        .flat_map(|method| method.params.iter());
    let functions = file
        .functions
        .iter()
        .filter(|function| {
            function.annotations.iter().any(|annotation| {
                annotation
                    .resolved_symbol
                    .as_ref()
                    .is_some_and(|symbol| symbol.0 == "dust_server::WS")
            })
        })
        .flat_map(|function| function.params.iter());
    methods
        .chain(functions)
        .filter_map(type_origin_request)
        .collect()
}
