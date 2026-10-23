use std::{collections::BTreeSet, path::Path};

use dust_diagnostics::Diagnostic;
use dust_ir::{ConfigApplicationIr, DartFileIr, SpanIr};

use super::{
    model::{ControllerPlan, HandlerPlan, VERBS},
    values::error,
};

/// Library helper tests.
#[cfg(test)]
mod tests;

/// Requires the generated part before emitting handler code.
pub(super) fn validate_part(file: &DartFileIr, diagnostics: &mut Vec<Diagnostic>) {
    let expected = Path::new(&file.output_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&file.output_path);
    if !file.part_directives.iter().any(|part| part.uri == expected) {
        diagnostics.push(error(
            file.span,
            "missing generated `part` directive for server handlers",
            format!("add `part '{expected}';`"),
        ));
    }
}

/// Rejects duplicate method-path pairs within one generated route group.
pub(super) fn validate_unique_routes(
    controllers: &[ControllerPlan<'_>],
    functions: &[HandlerPlan<'_>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for handlers in controllers
        .iter()
        .map(|value| value.handlers.as_slice())
        .chain(std::iter::once(functions))
    {
        let mut seen = BTreeSet::new();
        for handler in handlers {
            if !seen.insert((handler.method, handler.path.as_str())) {
                diagnostics.push(error(
                    handler.callable.span(),
                    "duplicate server route",
                    "change the HTTP method or path",
                ));
            }
        }
    }
}

/// Derives the top-level router factory from a source basename.
pub(super) fn default_routes_name(source_path: &str) -> String {
    let stem = Path::new(source_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("server");
    let singular = stem.strip_suffix('s').unwrap_or(stem);
    let mut result = String::new();
    let mut uppercase = false;
    for ch in singular.chars() {
        if ch == '_' || ch == '-' {
            uppercase = true;
        } else if uppercase {
            result.extend(ch.to_uppercase());
            uppercase = false;
        } else {
            result.push(ch);
        }
    }
    format!("{result}Routes")
}

/// Finds one resolved configuration by canonical symbol.
pub(super) fn find_config<'a>(
    configs: &'a [ConfigApplicationIr],
    symbol: &str,
) -> Option<&'a ConfigApplicationIr> {
    configs.iter().find(|value| value.symbol.0 == symbol)
}

/// Maps a server verb configuration to its HTTP method.
pub(super) fn verb_config(config: &ConfigApplicationIr) -> Option<&'static str> {
    VERBS
        .iter()
        .find(|(symbol, _)| *symbol == config.symbol.0)
        .map(|(_, method)| *method)
}

/// Returns whether a configuration belongs to the server namespace.
pub(super) fn is_server_config(config: &ConfigApplicationIr) -> bool {
    config.symbol.0.starts_with("dust_server::")
}

/// Finds the first server-owned annotation at any supported coordinate.
pub(super) fn first_server_span(file: &DartFileIr) -> Option<SpanIr> {
    file.library_annotations
        .iter()
        .find(|value| {
            value
                .resolved_symbol
                .as_ref()
                .is_some_and(|symbol| symbol.0.starts_with("dust_server::"))
        })
        .map(|value| value.span)
        .or_else(|| config_span(file))
        .or_else(|| {
            file.functions
                .iter()
                .flat_map(|value| value.annotations.iter())
                .find(|value| {
                    value
                        .resolved_symbol
                        .as_ref()
                        .is_some_and(|symbol| symbol.0.starts_with("dust_server::"))
                })
                .map(|value| value.span)
        })
        .or_else(|| {
            file.functions
                .iter()
                .flat_map(|value| value.params.iter())
                .flat_map(|value| value.configs.iter())
                .find(|value| value.symbol.0.starts_with("dust_server::"))
                .map(|value| value.span)
        })
}

/// Finds the first class, method, or method-parameter server config.
fn config_span(file: &DartFileIr) -> Option<SpanIr> {
    file.classes
        .iter()
        .flat_map(|value| value.configs.iter())
        .chain(
            file.classes
                .iter()
                .flat_map(|value| value.methods.iter())
                .flat_map(|value| {
                    value
                        .configs
                        .iter()
                        .chain(value.params.iter().flat_map(|param| param.configs.iter()))
                }),
        )
        .find(|value| value.symbol.0.starts_with("dust_server::"))
        .map(|value| value.span)
}
