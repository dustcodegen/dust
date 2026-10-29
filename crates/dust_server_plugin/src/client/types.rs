use std::collections::BTreeSet;

use dust_diagnostics::Diagnostic;
use dust_http_client_plugin::GeneratedEndpoint;
use dust_ir::{DartFileIr, ImportIr, TypeIr};

/// Dart core types that require no generated import.
const CORE_TYPES: &[&str] = &[
    "BigInt", "DateTime", "Duration", "Future", "Iterable", "List", "Map", "Never", "Object",
    "Set", "Stream", "String", "Symbol", "Uri", "bool", "double", "dynamic", "int", "num", "void",
];

/// Validates exposed types and returns the exact imports their prefixes use.
pub(super) fn required_imports(
    file: &DartFileIr,
    endpoints: &[GeneratedEndpoint],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<String> {
    let mut prefixes = BTreeSet::new();
    for endpoint in endpoints {
        collect_type(&endpoint.method.return_type, &mut prefixes, diagnostics);
        for param in &endpoint.method.params {
            collect_type(&param.ty, &mut prefixes, diagnostics);
        }
    }
    prefixes
        .into_iter()
        .filter_map(|prefix| import_for_prefix(file, &prefix, diagnostics))
        .collect()
}

/// Recursively records import prefixes or reports an unreachable type.
fn collect_type(ty: &TypeIr, prefixes: &mut BTreeSet<String>, diagnostics: &mut Vec<Diagnostic>) {
    match ty {
        TypeIr::Builtin { .. } | TypeIr::Dynamic => {}
        TypeIr::Named { name, args, .. } => {
            if let Some((prefix, member)) = name.split_once('.') {
                if member.starts_with('_') {
                    diagnostics.push(private_type(name));
                } else {
                    prefixes.insert(prefix.to_owned());
                }
            } else if name.starts_with('_') {
                diagnostics.push(private_type(name));
            } else if !CORE_TYPES.contains(&name.as_ref()) {
                diagnostics.push(Diagnostic::error(format!(
                    "generated client type `{name}` is not reachable through a prefixed package import"
                )));
            }
            for arg in args {
                collect_type(arg, prefixes, diagnostics);
            }
        }
        TypeIr::Function { .. } | TypeIr::Record { .. } | TypeIr::Unknown => {
            diagnostics.push(Diagnostic::error(
                "generated clients require named, fully resolved public types",
            ));
        }
    }
}

/// Builds the diagnostic for a private server-library type.
fn private_type(name: &str) -> Diagnostic {
    Diagnostic::error(format!(
        "generated client type `{name}` is private to the server library"
    ))
}

/// Resolves one used type prefix to exactly one source import.
fn import_for_prefix(
    file: &DartFileIr,
    prefix: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    let matches = file
        .import_directives
        .iter()
        .filter(|value| value.prefix.as_deref() == Some(prefix))
        .collect::<Vec<_>>();
    let import = match matches.as_slice() {
        [value] => *value,
        [] => {
            diagnostics.push(Diagnostic::error(format!(
                "generated client type prefix `{prefix}` has no source import"
            )));
            return None;
        }
        _ => {
            diagnostics.push(Diagnostic::error(format!(
                "generated client type prefix `{prefix}` is ambiguous"
            )));
            return None;
        }
    };
    render_import(import, diagnostics)
}

/// Renders a safe package import for a client-visible prefixed type.
fn render_import(import: &ImportIr, diagnostics: &mut Vec<Diagnostic>) -> Option<String> {
    if !import.uri.starts_with("package:") || import.uri.starts_with("package:dust_server/") {
        diagnostics.push(Diagnostic::error(format!(
            "generated client cannot expose types from `{}`",
            import.uri
        )));
        return None;
    }
    if import.is_deferred || !import.hide.is_empty() || !import.show.is_empty() {
        diagnostics.push(Diagnostic::error(format!(
            "generated client import `{}` must be a plain prefixed import",
            import.uri
        )));
        return None;
    }
    Some(format!(
        "import '{}' as {};",
        import.uri,
        import.prefix.as_deref().unwrap_or_default()
    ))
}
