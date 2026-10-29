/// Route-fact to shared HTTP-emitter conversion.
mod endpoint;
/// Public type reachability and import selection.
mod types;

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use dust_diagnostics::Diagnostic;
use dust_http_client_plugin::{GeneratedClient, generate_client_library};
use dust_ir::DartFileIr;
use dust_plugin_api::{GENERATED_HEADER, WorkspaceAnalysis};

use crate::{
    ServerClientDestination,
    analysis::route_facts,
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
) -> Result<Vec<(PathBuf, String)>, Vec<Diagnostic>> {
    let facts = route_facts(analysis, &file.source_path)
        .map_err(|error| vec![error])?
        .into_iter()
        .filter(|fact| {
            destination.tags.is_empty()
                || fact.tags.iter().any(|tag| destination.tags.contains(tag))
        })
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
    let parent = main_output.parent().unwrap_or(Path::new(""));
    let main_stem = main_output
        .file_stem()
        .and_then(|value| value.to_str())
        .and_then(|value| value.strip_suffix(".g"))
        .unwrap();
    let mut outputs = Vec::new();
    let mut exports = Vec::new();
    for (controller, group) in groups {
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
            match endpoint::build_endpoint(fact, handler) {
                Ok(endpoint) => endpoints.push(endpoint),
                Err(error) => diagnostics.push(error),
            }
        }
        let imports = types::required_imports(file, &endpoints, &mut diagnostics);
        let (class_name, suffix) = group_identity(controller.as_deref(), &file.source_path)?;
        let file_name = format!("{main_stem}.{suffix}.g.dart");
        let source = generate_client_library(&GeneratedClient {
            class_name,
            imports,
            endpoints,
        })
        .map_err(|error| vec![error])?;
        exports.push(format!("export '{file_name}';"));
        outputs.push((parent.join(file_name), source));
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

/// Converts one route file stem into a Dart class-name prefix.
fn file_class_name(source: &str) -> Result<String, Vec<Diagnostic>> {
    let stem = std::path::Path::new(source)
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| vec![Diagnostic::error("server source has no UTF-8 file name")])?;
    let name = stem
        .split(|value: char| !value.is_ascii_alphanumeric())
        .filter(|value| !value.is_empty())
        .map(|value| {
            let mut chars = value.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<String>();
    Ok(name)
}

/// Selects the class name and file suffix for one controller or route module.
fn group_identity(
    controller: Option<&str>,
    source: &str,
) -> Result<(String, String), Vec<Diagnostic>> {
    match controller {
        Some(name) => Ok((format!("{name}Client"), snake_case(name))),
        None => {
            let name = file_class_name(source)?;
            Ok((format!("{name}RoutesClient"), "routes".to_owned()))
        }
    }
}

/// Converts a Dart class name into a deterministic file suffix.
fn snake_case(value: &str) -> String {
    let mut output = String::new();
    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() && index > 0 {
            output.push('_');
        }
        output.push(ch.to_ascii_lowercase());
    }
    output
}

/// Returns the generated barrel path inside the configured client package.
fn main_output_path(
    file: &DartFileIr,
    destination: &ServerClientDestination,
) -> Result<PathBuf, Vec<Diagnostic>> {
    let source = std::path::Path::new(&file.source_path);
    let library_root = std::path::Path::new(&file.package_root).join("lib");
    let relative = source
        .strip_prefix(&library_root)
        .or_else(|_| source.strip_prefix("lib"))
        .map_err(|_| {
            vec![Diagnostic::error(
                "generated server clients require the route source to be under `lib/`",
            )]
        })?;
    let stem = relative
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| vec![Diagnostic::error("server source has no UTF-8 file name")])?;
    let mut output = destination
        .package_root
        .join("lib/src/generated")
        .join(relative);
    output.set_file_name(format!("{stem}_client.g.dart"));
    Ok(output)
}
