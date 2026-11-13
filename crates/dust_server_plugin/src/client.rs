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
use dust_http_client_plugin::{GeneratedClient, generate_client_library_with_ws};
use dust_ir::DartFileIr;
use dust_plugin_api::{GENERATED_HEADER, WorkspaceAnalysis};

use output::{accepts_tags, main_output_path, plan_group_outputs};

pub(crate) use output::collect_output_claims;
pub use output::is_client_output_claimed;

use crate::{
    ServerClientDestination,
    analysis::{PayloadCapabilityIndex, response_serde_types, route_facts, ws_facts},
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
    let socket_facts = ws_facts(analysis, &file.source_path)
        .map_err(|error| vec![error])?
        .into_iter()
        .filter(|fact| accepts_tags(destination, &fact.route.tags))
        .collect::<Vec<_>>();
    if facts.is_empty() && socket_facts.is_empty() {
        return Ok(Vec::new());
    }
    let mut diagnostics = Vec::new();
    let mut groups = BTreeMap::<Option<String>, (Vec<_>, Vec<_>)>::new();
    for fact in &facts {
        groups
            .entry(fact.controller.clone())
            .or_default()
            .0
            .push(fact);
    }
    for fact in &socket_facts {
        groups
            .entry(fact.route.controller.clone())
            .or_default()
            .1
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
    for ((group, socket_group), (class_name, output_path)) in
        groups.into_values().zip(group_outputs)
    {
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
        let mut sockets = Vec::new();
        if !socket_group.is_empty() {
            let prefix = types::runtime_prefix(
                file,
                &class_name,
                names.iter().map(String::as_str).chain(
                    socket_group
                        .iter()
                        .map(|fact| fact.route.operation_id.as_str()),
                ),
            );
            for fact in socket_group {
                if !names.insert(fact.route.operation_id.clone()) {
                    diagnostics.push(Diagnostic::error(format!(
                        "generated client operation `{}` is declared more than once in one route group",
                        fact.route.operation_id
                    )));
                    continue;
                }
                let Some(handler) =
                    find_handler(plan, fact.route.controller.as_deref(), &fact.route.callable)
                else {
                    diagnostics.push(Diagnostic::error(format!(
                        "route fact `{}` no longer matches a server handler",
                        fact.route.operation_id
                    )));
                    continue;
                };
                match endpoint::build_ws_endpoint(fact, handler, &prefix) {
                    Ok(endpoint) => sockets.push(endpoint),
                    Err(error) => diagnostics.push(error),
                }
            }
        }
        let imports = if sockets.is_empty() {
            types::required_imports(file, &endpoints, &mut diagnostics)
        } else {
            types::required_imports_with_ws(
                file,
                &class_name,
                &endpoints,
                &sockets,
                &mut diagnostics,
            )
        };
        let source = generate_client_library_with_ws(
            &GeneratedClient {
                class_name,
                imports,
                endpoints,
            },
            &sockets,
        )
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
