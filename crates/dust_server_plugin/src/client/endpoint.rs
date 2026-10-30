use dust_diagnostics::Diagnostic;
use dust_http_client_plugin::{
    GeneratedEndpoint, GeneratedHttpVerb, GeneratedParam, GeneratedRequestMode,
};
use dust_ir::{MethodIr, MethodParamIr, TypeIr};

use crate::{
    analysis::{ServerParamFact, ServerRouteFact},
    plan::{Callable, ExtractionWrapper, HandlerPlan, ParamPlan},
};

#[cfg(test)]
mod tests;

/// Builds one standalone HTTP endpoint from matching fact and typed-plan views.
pub(super) fn build_endpoint(
    fact: &ServerRouteFact,
    handler: &HandlerPlan<'_>,
    serde_response_types: Vec<String>,
) -> Result<GeneratedEndpoint, Diagnostic> {
    let mut params = Vec::new();
    let mut bindings = Vec::new();
    for fact_param in &fact.params {
        let plan = handler
            .params
            .iter()
            .find(|value| value.param.name == fact_param.name)
            .ok_or_else(|| {
                Diagnostic::error(format!(
                    "route fact parameter `{}` no longer matches `{}`",
                    fact_param.name, fact.operation_id
                ))
            })?;
        let Some(binding) = binding(fact_param, plan, params.len())? else {
            continue;
        };
        params.push(client_param(plan));
        bindings.push(binding);
    }
    let request_mode = if fact.params.iter().any(|value| value.binding == "part") {
        GeneratedRequestMode::MultiPart
    } else if fact.params.iter().any(|value| value.binding == "field") {
        GeneratedRequestMode::FormUrlEncoded
    } else {
        GeneratedRequestMode::Standard
    };
    Ok(GeneratedEndpoint {
        method: MethodIr {
            name: fact.operation_id.clone(),
            is_static: false,
            is_external: false,
            return_type: client_return_type(handler.callable.return_type())?,
            has_body: false,
            body_source: None,
            params,
            span: match handler.callable {
                Callable::Method(value) => value.span,
                Callable::Function(value) => value.span,
            },
            traits: Vec::new(),
            configs: Vec::new(),
        },
        verb: verb(&fact.method)?,
        path: join_path(&fact.prefix, &fact.path),
        request_mode,
        params: bindings,
        serde_response_types,
    })
}

/// Removes server annotations and unwraps extractor-only parameter wrappers.
fn client_param(plan: &ParamPlan<'_>) -> MethodParamIr {
    let mut param = plan.param.clone();
    param.ty = match plan.wrapper {
        ExtractionWrapper::Optional => plan.value_type.clone().nullable(),
        ExtractionWrapper::Required | ExtractionWrapper::Fallible => plan.value_type.clone(),
    };
    param.configs.clear();
    param.traits.clear();
    param.has_default = false;
    param.default_value_source = None;
    if plan.wrapper == ExtractionWrapper::Optional {
        param.is_required = false;
    }
    param
}

/// Maps one server extraction fact to its client-visible wire binding.
fn binding(
    fact: &ServerParamFact,
    plan: &ParamPlan<'_>,
    index: usize,
) -> Result<Option<GeneratedParam>, Diagnostic> {
    let key = || {
        fact.binding_name
            .clone()
            .unwrap_or_else(|| fact.name.clone())
    };
    Ok(match fact.binding.as_str() {
        "path" => Some(GeneratedParam::Path { index, key: key() }),
        "query" | "query-list" => Some(GeneratedParam::Query { index, key: key() }),
        "queries" if is_string_map(plan.value_type) => Some(GeneratedParam::Queries { index }),
        "query-model" => Some(GeneratedParam::QueryModel { index }),
        "queries" => {
            return Err(Diagnostic::error(format!(
                "generated client parameter `{}` uses an aggregate @Queries value that cannot be encoded; use Map<String, String> or @Query() with a shared Serde model",
                fact.name
            )));
        }
        "header" => Some(GeneratedParam::Header { index, key: key() }),
        "header-map" => Some(GeneratedParam::HeaderMap { index }),
        "body" => Some(GeneratedParam::Body { index }),
        "field" => Some(GeneratedParam::Field { index, key: key() }),
        "part" => Some(GeneratedParam::Part { index, key: key() }),
        "custom" | "raw-query" | "raw-body" | "state" | "peer" | "request" | "request-parts" => {
            None
        }
        other => {
            return Err(Diagnostic::error(format!(
                "generated client parameter `{}` has unknown binding `{other}`",
                fact.name
            )));
        }
    })
}

/// Converts a normalized server method into the shared emitter verb.
fn verb(method: &str) -> Result<GeneratedHttpVerb, Diagnostic> {
    match method {
        "GET" => Ok(GeneratedHttpVerb::Get),
        "POST" => Ok(GeneratedHttpVerb::Post),
        "PUT" => Ok(GeneratedHttpVerb::Put),
        "PATCH" => Ok(GeneratedHttpVerb::Patch),
        "DELETE" => Ok(GeneratedHttpVerb::Delete),
        "HEAD" => Ok(GeneratedHttpVerb::Head),
        "OPTIONS" => Ok(GeneratedHttpVerb::Options),
        other => Err(Diagnostic::error(format!(
            "generated client route uses unsupported HTTP method `{other}`"
        ))),
    }
}

/// Normalizes a server response contract into an asynchronous client result.
fn client_return_type(server: &TypeIr) -> Result<TypeIr, Diagnostic> {
    let value = if server.is_named("Future") && server.args().len() == 1 {
        &server.args()[0]
    } else {
        server
    };
    let value = match value.name() {
        Some("Result") if value.args().len() == 2 => value.args()[0].clone(),
        Some("Option") if value.args().len() == 1 => value.args()[0].clone().nullable(),
        Some("Unit" | "void") => TypeIr::named("void"),
        Some("Response") => {
            return Err(Diagnostic::error(
                "generated clients cannot infer a payload type from a shelf Response",
            ));
        }
        _ => value.clone(),
    };
    Ok(TypeIr::generic("Future", vec![value]))
}

/// Returns whether a whole-query value can be merged directly on the client.
fn is_string_map(ty: &TypeIr) -> bool {
    ty.is_named("Map")
        && ty.args().len() == 2
        && ty.args()[0].is_named("String")
        && ty.args()[1].is_named("String")
}

/// Joins route-group and handler paths with exactly one boundary slash.
fn join_path(prefix: &str, path: &str) -> String {
    if prefix.is_empty() {
        return path.to_owned();
    }
    format!(
        "{}/{}",
        prefix.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}
