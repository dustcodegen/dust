use dust_ir::DartFileIr;
use dust_plugin_api::{PluginRegistry, WorkspaceAnalysisBuilder};
use dust_resolver::BindingContext;

/// Resolve only the references an executing plugin requested in this shared pass.
pub(super) fn collect<'a>(
    file: &DartFileIr,
    registry: &PluginRegistry,
    bindings: impl FnOnce() -> BindingContext<'a>,
    analysis: &mut WorkspaceAnalysisBuilder,
) {
    let requests = registry.type_origin_requests(file);
    if requests.is_empty() {
        return;
    }
    let bindings = bindings();
    for request in requests {
        if let Some(origin) = bindings.type_declaration_origin(&request.name, request.span) {
            analysis.add_type_origin(
                &file.source_path,
                &request,
                &origin.library_uri,
                &origin.name,
            );
        }
    }
}

#[cfg(test)]
mod tests;
