/// Route-fact to shared HTTP-emitter conversion.
mod endpoint;
/// Client output naming and workspace ownership checks.
mod output;
/// Public type reachability and import selection.
mod types;

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use dust_diagnostics::Diagnostic;
use dust_http_client_plugin::{GeneratedClient, generate_client_library};
use dust_ir::DartFileIr;
use dust_plugin_api::{GENERATED_HEADER, WorkspaceAnalysis};

use output::{accepts_tags, main_output_path, plan_group_outputs};

pub(crate) use output::collect_output_claims;
pub use output::is_client_output_claimed;

use crate::{
    ServerClientDestination,
    analysis::{PayloadCapabilityIndex, response_serde_types, route_facts},
    plan::{HandlerPlan, ServerPlan},
};

#[cfg(test)]
mod tests;

/// Generates one standalone client library for one server route library.
pub(crate) fn generate_server_client(
    file: &DartFileIr,
    plan: &ServerPlan<'_>,
    destination: &ServerClientDestination,
    analysis: &WorkspaceAnalysis,
    payload_models: &PayloadCapabilityIndex,
) -> Result<Vec<(PathBuf, String)>, Vec<Diagnostic>> {
    let facts = route_facts(analysis, &file.source_path)
        .map_err(|error| vec![error])?
        .into_iter()
        .filter(|fact| accepts_tags(destination, &fact.tags))
        .collect::<Vec<_>>();
    if facts.is_empty() {
        return Ok(Vec::new());
    }
    let mut diagnostics = Vec::new();
    let mut groups = BTreeMap::<Option<String>, Vec<_>>::new();
    for fact in &facts {
        groups
            .entry(fact.controller.clone())
            .or_default()
            .push(fact);
    }
    let main_output = main_output_path(file, destination)?;
    let group_outputs = plan_group_outputs(
        file,
        plan,
        &main_output,
        groups.keys().map(Option::as_deref),
        analysis,
    )?;
    let mut outputs = Vec::new();
    let mut exports = Vec::new();
    for (group, (class_name, output_path)) in groups.into_values().zip(group_outputs) {
        let mut names = BTreeSet::new();
        let mut endpoints = Vec::new();
        for fact in group {
            if !names.insert(fact.operation_id.clone()) {
                diagnostics.push(Diagnostic::error(format!(
                    "generated client operation `{}` is declared more than once in one route group",
                    fact.operation_id
                )));
                continue;
            }
            let Some(handler) = find_handler(plan, fact.controller.as_deref(), &fact.callable)
            else {
                diagnostics.push(Diagnostic::error(format!(
                    "route fact `{}` no longer matches a server handler",
                    fact.operation_id
                )));
                continue;
            };
            let serde_response_types = match response_serde_types(
                file,
                handler.callable.return_type(),
                handler.callable.span(),
                payload_models,
            ) {
                Ok(value) => value,
                Err(errors) => {
                    diagnostics.extend(errors);
                    continue;
                }
            };
            match endpoint::build_endpoint(fact, handler, serde_response_types) {
                Ok(endpoint) => endpoints.push(endpoint),
                Err(error) => diagnostics.push(error),
            }
        }
        let imports = types::required_imports(file, &endpoints, &mut diagnostics);
        let source = generate_client_library(&GeneratedClient {
            class_name,
            imports,
            endpoints,
        })
        .map_err(|error| vec![error])?;
        let file_name = output_path.file_name().unwrap().to_str().unwrap();
        exports.push(format!("export '{file_name}';"));
        outputs.push((output_path, source));
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    exports.sort();
    outputs.push((
        main_output,
        format!("{GENERATED_HEADER}\n{}\n", exports.join("\n")),
    ));
    Ok(outputs)
}

/// Matches a serialized route fact back to its typed server plan.
fn find_handler<'a>(
    plan: &'a ServerPlan<'a>,
    controller: Option<&str>,
    callable: &str,
) -> Option<&'a HandlerPlan<'a>> {
    match controller {
        Some(name) => plan
            .controllers
            .iter()
            .find(|value| value.class.name == name)?
            .handlers
            .iter()
            .find(|value| value.callable.name() == callable),
        None => plan
            .functions
            .iter()
            .find(|value| value.callable.name() == callable),
    }
}
