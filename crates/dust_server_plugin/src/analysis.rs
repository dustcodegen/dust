use dust_dart_emit::DYNAMIC_TYPES;
use dust_ir::{DartFileIr, TypeIr};
use dust_plugin_api::{WorkspaceAnalysis, WorkspaceAnalysisBuilder};
use serde::{Deserialize, Serialize};

use crate::plan::{Extraction, ExtractionWrapper, HandlerPlan, ServerPlan, plan_server};

#[cfg(test)]
mod tests;

/// Versioned workspace-analysis bucket for generated server route facts.
pub(crate) const SERVER_ROUTES_ANALYSIS_KEY: &str = "dust_server.routes.v1";

#[derive(Debug, Deserialize, Serialize)]
/// Canonical, serializable facts for one generated server route.
pub(crate) struct ServerRouteFact {
    /// Source library that declares the handler.
    pub(crate) source_path: String,
    /// Controller name, or `None` for a top-level handler.
    pub(crate) controller: Option<String>,
    /// Prefix inherited from the controller or module.
    pub(crate) prefix: String,
    /// Dart callable that implements the route.
    pub(crate) callable: String,
    /// Normalized HTTP method.
    pub(crate) method: String,
    /// Route-local path.
    pub(crate) path: String,
    /// Declared success status.
    status: u16,
    /// Stable operation identifier.
    pub(crate) operation_id: String,
    /// Optional short documentation summary.
    summary: Option<String>,
    /// Optional long documentation description.
    description: Option<String>,
    /// Ordered controller and route tags.
    pub(crate) tags: Vec<String>,
    /// Whether consumers should mark the route deprecated.
    deprecated: bool,
    /// Whether documentation consumers should hide the route.
    hidden: bool,
    /// Structured handler parameters in declaration order.
    pub(crate) params: Vec<ServerParamFact>,
    /// Rendered Dart return type.
    return_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
/// Canonical, serializable facts for one handler parameter.
pub(crate) struct ServerParamFact {
    /// Dart parameter name.
    pub(crate) name: String,
    /// Rendered Dart parameter type.
    type_source: String,
    /// Normalized extractor kind.
    pub(crate) binding: String,
    /// Optional path, query, header, field, part, or custom binding name.
    pub(crate) binding_name: Option<String>,
    /// Required, optional, or fallible extraction wrapper.
    wrapper: String,
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
        binding: binding.to_owned(),
        binding_name,
        wrapper: match param.wrapper {
            ExtractionWrapper::Required => "required",
            ExtractionWrapper::Optional => "optional",
            ExtractionWrapper::Fallible => "fallible",
        }
        .to_owned(),
    }
}

/// Reads this library's canonical route facts from the shared workspace analysis.
pub(crate) fn route_facts(
    analysis: &WorkspaceAnalysis,
    source_path: &str,
) -> Result<Vec<ServerRouteFact>, dust_diagnostics::Diagnostic> {
    analysis
        .string_set(SERVER_ROUTES_ANALYSIS_KEY)
        .unwrap_or_default()
        .iter()
        .map(|value| {
            serde_json::from_str::<ServerRouteFact>(value).map_err(|error| {
                dust_diagnostics::Diagnostic::error(format!(
                    "invalid {SERVER_ROUTES_ANALYSIS_KEY} route fact: {error}"
                ))
            })
        })
        .filter_map(|result| match result {
            Ok(fact) if fact.source_path == source_path => Some(Ok(fact)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

/// Returns the normalized extractor kind and optional source name.
fn binding(extraction: &Extraction) -> (&'static str, Option<String>) {
    match extraction {
        Extraction::Path(name) => ("path", Some(name.clone())),
        Extraction::Query(name) => ("query", Some(name.clone())),
        Extraction::QueryList(name) => ("query-list", Some(name.clone())),
        Extraction::QueryModel(_) => ("query-model", None),
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
