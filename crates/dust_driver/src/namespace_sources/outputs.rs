use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::PathBuf,
};

use dust_diagnostics::Diagnostic;
use dust_ir::SymbolId;
use dust_plugin_api::PluginRegistry;
use dust_resolver::{NamespaceBinding, NamespaceGraph, SymbolCatalog};

use super::{graph, index::Source, loader::Unit};

/// Invariant failures and source identities not constructed by normal discovery.
#[cfg(test)]
mod tests;

/// One potential library's handwritten source and canonical class producer.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Claim {
    /// Canonical identity of the handwritten declaring library.
    source: String,
    /// Actual canonical class annotation required by the plugin.
    producer: SymbolId,
}

/// Computes paths without inferring ownership from any filename or spelling.
pub(super) fn plan(
    roots: &[Source],
    registry: &PluginRegistry,
) -> Result<BTreeMap<PathBuf, Claim>, Diagnostic> {
    let mut claims = BTreeMap::new();
    for root in roots {
        for output in registry.potential_generated_libraries(&root.path)? {
            let path = crate::source_uri::normalize_path(&output.path);
            let claim = Claim {
                source: root.uri.clone(),
                producer: output.producer,
            };
            if let Some(previous) = claims.insert(path.clone(), claim)
                && claims[&path] != previous
            {
                return Err(Diagnostic::error(format!(
                    "generated library `{}` has conflicting producers",
                    path.display()
                )));
            }
        }
    }
    Ok(claims)
}

/// Removes unsupported placeholders monotonically and rechecks surviving owners.
pub(super) fn verify(
    units: &mut BTreeMap<String, Result<Unit, String>>,
    claims: &BTreeMap<PathBuf, Claim>,
    catalog: &SymbolCatalog,
) -> Result<NamespaceGraph, Diagnostic> {
    let mut eligible: BTreeSet<_> = claims.keys().cloned().collect();
    let graph = loop {
        let graph = graph::build(units)?;
        let mut ownership = HashMap::new();
        let rejected: BTreeSet<_> = eligible
            .iter()
            .filter(|path| {
                let claim = &claims[*path];
                !*ownership
                    .entry((&claim.source, &claim.producer))
                    .or_insert_with(|| owned(claim, units, &graph, catalog))
            })
            .cloned()
            .collect();
        if rejected.is_empty() {
            break graph;
        }
        for path in &rejected {
            eligible.remove(path);
        }
        let mut changed_units = false;
        for result in units.values_mut() {
            if let Ok(unit) = result
                && unit.generated_library
                && rejected.contains(&unit.source.path)
            {
                let path = &unit.source.path;
                *result = Err(format!(
                    "generated library `{}` has no verified handwritten class producer `{}`",
                    path.display(),
                    claims[path].producer.0
                ));
                changed_units = true;
            }
        }
        // Rebuilding drops requests made while checking claims we just rejected.
        if !changed_units && graph.required_lexical_imports().is_empty() {
            break graph;
        }
    };
    for unit in units.values().filter_map(|unit| unit.as_ref().ok()) {
        if eligible.contains(&unit.source.path) && !unit.generated_library {
            return Err(Diagnostic::error(format!(
                "generated library output `{}` collides with an unmarked existing source",
                unit.source.path.display()
            )));
        }
    }
    Ok(graph)
}

/// Only actual handwritten class metadata can authorize a declared output.
fn owned(
    claim: &Claim,
    units: &BTreeMap<String, Result<Unit, String>>,
    graph: &NamespaceGraph,
    catalog: &SymbolCatalog,
) -> bool {
    let Some(Ok(unit)) = units.get(&claim.source) else {
        return false;
    };
    if unit.generated_library
        || !unit.namespace_diagnostics.is_empty()
        || unit
            .parsed
            .directives
            .iter()
            .any(|directive| matches!(directive, dust_parser_dart::ParsedDirective::PartOf { .. }))
    {
        return false;
    }
    unit.parsed.classes.iter().flat_map(|class| &class.annotations).any(|annotation| {
        let name = annotation.prefix.as_ref().map_or_else(|| annotation.name.clone(), |prefix| format!("{prefix}.{}", annotation.name));
        matches!(graph.scope(&claim.source).bind_at(&name, catalog, dust_ir::SpanIr::new(unit.file_id, annotation.span)), NamespaceBinding::Owned(owner) if owner == claim.producer)
    })
}
