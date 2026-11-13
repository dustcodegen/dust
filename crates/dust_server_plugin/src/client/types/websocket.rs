use dust_dart_emit::{NameAllocator, library_reference_names};
use dust_diagnostics::SourceLabel;
use dust_http_client_plugin::GeneratedWsEndpoint;

use super::*;
use crate::plan::is_emittable_identifier;

/// Focused WebSocket regression tests.
#[cfg(test)]
mod tests;

/// Chooses a runtime prefix that source names and generated operations cannot capture.
pub(crate) fn runtime_prefix<'a>(
    file: &DartFileIr,
    class_name: &str,
    operation_names: impl IntoIterator<Item = &'a str>,
) -> String {
    let mut reserved = library_reference_names(file);
    reserved.insert(class_name.to_owned());
    reserved.extend(operation_names.into_iter().map(str::to_owned));
    reserved.extend(
        file.import_directives
            .iter()
            .filter_map(|import| import.prefix.clone()),
    );
    reserved.extend(
        file.functions
            .iter()
            .flat_map(|function| function.params.iter())
            .map(|param| param.name.clone()),
    );
    reserved.extend(
        file.classes
            .iter()
            .flat_map(|class| class.methods.iter())
            .flat_map(|method| method.params.iter())
            .map(|param| param.name.clone()),
    );
    NameAllocator::new(reserved).allocate("dustWs")
}

/// Collects real DTO and handshake types separately from the synthetic socket result.
pub(crate) fn required_imports_with_ws(
    file: &DartFileIr,
    class_name: &str,
    endpoints: &[GeneratedEndpoint],
    sockets: &[GeneratedWsEndpoint],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<String> {
    let mut prefixes = BTreeSet::new();
    for endpoint in endpoints {
        collect_type(&endpoint.method.return_type, &mut prefixes, diagnostics);
        for param in &endpoint.method.params {
            collect_type(&param.ty, &mut prefixes, diagnostics);
        }
    }
    for endpoint in sockets {
        collect_type(&endpoint.send_type, &mut prefixes, diagnostics);
        collect_type(&endpoint.receive_type, &mut prefixes, diagnostics);
        for param in &endpoint.method.params {
            collect_type(&param.ty, &mut prefixes, diagnostics);
        }
    }
    let methods = endpoints
        .iter()
        .map(|endpoint| &endpoint.method)
        .chain(sockets.iter().map(|endpoint| &endpoint.method))
        .collect::<Vec<_>>();
    let reserved = [
        "Uri",
        "_dio",
        "_baseUrl",
        "_webSocketConnector",
        "_webSocketUri",
        "_combineBaseUrls",
    ];
    for method in &methods {
        if method.name == class_name || reserved.contains(&method.name.as_str()) {
            diagnostics.push(
                Diagnostic::error(format!(
                    "generated client operation `{}` conflicts with a generated client member",
                    method.name
                ))
                .with_label(SourceLabel::new(
                    method.span.file_id,
                    method.span.range,
                    "rename the client operation",
                )),
            );
        }
        for param in &method.params {
            if ["_uri", "_headers", "_queryParameters"].contains(&param.name.as_str())
                || reserved.contains(&param.name.as_str())
            {
                diagnostics.push(
                    Diagnostic::error(format!(
                        "generated client parameter `{}` conflicts with generated WebSocket setup",
                        param.name
                    ))
                    .with_label(SourceLabel::new(
                        param.span.file_id,
                        param.span.range,
                        "rename the handshake parameter",
                    )),
                );
            }
        }
    }
    prefixes.into_iter().filter_map(|prefix| {
        if !is_emittable_identifier(&prefix) || prefix == class_name || CORE_TYPES.contains(&prefix.as_str())
            || prefix == "Dio" || prefix == "RequestOptions" || reserved.contains(&prefix.as_str())
            || methods.iter().any(|method| method.name == prefix || method.params.iter().any(|param| param.name == prefix)) {
            let span = file.import_directives.iter().find(|import| import.prefix.as_deref() == Some(prefix.as_str())).map_or(file.span, |import| import.span);
            diagnostics.push(Diagnostic::error(format!("generated client type prefix `{prefix}` conflicts with a generated declaration or parameter")).with_label(SourceLabel::new(span.file_id, span.range, "use an unambiguous import prefix")));
            return None;
        }
        if let Some(import) = file.import_directives.iter().find(|import| {
            import.prefix.as_deref() == Some(prefix.as_str())
                && import.namespace.as_ref().is_some_and(|namespace| !namespace.conditional_uris.is_empty())
        }) {
            diagnostics.push(Diagnostic::error(format!(
                "generated WebSocket client import `{}` cannot use conditional branches", import.uri
            )).with_label(SourceLabel::new(import.span.file_id, import.span.range,
                "use one plain prefixed package import for shared WebSocket models")));
            return None;
        }
        import_for_prefix(file, &prefix, diagnostics)
    }).collect()
}
