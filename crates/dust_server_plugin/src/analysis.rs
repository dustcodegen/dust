use dust_dart_emit::DYNAMIC_TYPES;
use dust_ir::{DartFileIr, TypeIr};
use dust_plugin_api::WorkspaceAnalysisBuilder;
use serde::Serialize;

use crate::plan::{Extraction, ExtractionWrapper, HandlerPlan, ServerPlan, plan_server};

#[cfg(test)]
mod tests;

/// Versioned workspace-analysis bucket for generated server route facts.
pub(crate) const SERVER_ROUTES_ANALYSIS_KEY: &str = "dust_server.routes.v1";

#[derive(Serialize)]
/// Canonical, serializable facts for one generated server route.
struct ServerRouteFact {
    /// Source library that declares the handler.
    source_path: String,
    /// Controller name, or `None` for a top-level handler.
    controller: Option<String>,
    /// Prefix inherited from the controller or module.
    prefix: String,
    /// Dart callable that implements the route.
    callable: String,
    /// Normalized HTTP method.
    method: String,
    /// Route-local path.
    path: String,
    /// Declared success status.
    status: u16,
    /// Stable operation identifier.
    operation_id: String,
    /// Optional short documentation summary.
    summary: Option<String>,
    /// Optional long documentation description.
    description: Option<String>,
    /// Ordered controller and route tags.
    tags: Vec<String>,
    /// Whether consumers should mark the route deprecated.
    deprecated: bool,
    /// Whether documentation consumers should hide the route.
    hidden: bool,
    /// Structured handler parameters in declaration order.
    params: Vec<ServerParamFact>,
    /// Rendered Dart return type.
    return_type: String,
}

#[derive(Serialize)]
/// Canonical, serializable facts for one handler parameter.
struct ServerParamFact {
    /// Dart parameter name.
    name: String,
    /// Rendered Dart parameter type.
    type_source: String,
    /// Normalized extractor kind.
    binding: &'static str,
    /// Optional path, query, header, field, part, or custom binding name.
    binding_name: Option<String>,
    /// Required, optional, or fallible extraction wrapper.
    wrapper: &'static str,
}

/// Collects valid generated route facts from canonical IR.
pub(crate) fn collect_server_workspace_analysis_ir(
    file: &DartFileIr,
    analysis: &mut WorkspaceAnalysisBuilder,
) {
    let Ok(Some(plan)) = plan_server(file) else {
        return;
    };
    collect_plan(file, &plan, analysis);
}

/// Adds controller and top-level route facts from a validated server plan.
fn collect_plan(file: &DartFileIr, plan: &ServerPlan<'_>, analysis: &mut WorkspaceAnalysisBuilder) {
    for controller in &plan.controllers {
        for handler in &controller.handlers {
            add_fact(
                file,
                Some(controller.class.name.clone()),
                &controller.prefix,
                handler,
                analysis,
            );
        }
    }
    for handler in &plan.functions {
        add_fact(file, None, &plan.function_prefix, handler, analysis);
    }
}

/// Serializes and records one handler fact in the versioned analysis bucket.
fn add_fact(
    file: &DartFileIr,
    controller: Option<String>,
    prefix: &str,
    handler: &HandlerPlan<'_>,
    analysis: &mut WorkspaceAnalysisBuilder,
) {
    let fact = ServerRouteFact {
        source_path: file.source_path.clone(),
        controller,
        prefix: prefix.to_owned(),
        callable: handler.callable.name().to_owned(),
        method: handler.method.to_owned(),
        path: handler.path.clone(),
        status: handler.status,
        operation_id: handler.metadata.operation_id.clone(),
        summary: handler.metadata.summary.clone(),
        description: handler.metadata.description.clone(),
        tags: handler.metadata.tags.clone(),
        deprecated: handler.metadata.deprecated,
        hidden: handler.metadata.hidden,
        params: handler.params.iter().map(param_fact).collect(),
        return_type: render_type(handler.callable.return_type()),
    };
    if let Ok(value) = serde_json::to_string(&fact) {
        analysis.add_string_set_value(SERVER_ROUTES_ANALYSIS_KEY, value);
    }
}

/// Converts one validated parameter plan into its public analysis shape.
fn param_fact(param: &crate::plan::ParamPlan<'_>) -> ServerParamFact {
    let (binding, binding_name) = binding(&param.extraction);
    ServerParamFact {
        name: param.param.name.clone(),
        type_source: render_type(&param.param.ty),
        binding,
        binding_name,
        wrapper: match param.wrapper {
            ExtractionWrapper::Required => "required",
            ExtractionWrapper::Optional => "optional",
            ExtractionWrapper::Fallible => "fallible",
        },
    }
}

/// Returns the normalized extractor kind and optional source name.
fn binding(extraction: &Extraction) -> (&'static str, Option<String>) {
    match extraction {
        Extraction::Path(name) => ("path", Some(name.clone())),
        Extraction::Query(name) => ("query", Some(name.clone())),
        Extraction::QueryList(name) => ("query-list", Some(name.clone())),
        Extraction::Queries => ("queries", None),
        Extraction::Header(name) => ("header", Some(name.clone())),
        Extraction::HeaderMap => ("header-map", None),
        Extraction::Body(_) => ("body", None),
        Extraction::Custom(name) => ("custom", Some(name.clone())),
        Extraction::RawQuery => ("raw-query", None),
        Extraction::Field(name) => ("field", Some(name.clone())),
        Extraction::Part { name, .. } => ("part", Some(name.clone())),
        Extraction::RawBody(_) => ("raw-body", None),
        Extraction::State => ("state", None),
        Extraction::Peer => ("peer", None),
        Extraction::Request => ("request", None),
        Extraction::RequestParts => ("request-parts", None),
    }
}

/// Renders a Dart type without replacing unknown types.
fn render_type(ty: &TypeIr) -> String {
    DYNAMIC_TYPES.render(ty)
}
