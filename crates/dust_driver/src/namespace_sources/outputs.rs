use std::{
    collections::{BTreeMap, BTreeSet},
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
            if let Some(previous) = claims.insert(path.clone(), claim) {
                if claims[&path] != previous {
                    return Err(Diagnostic::error(format!(
                        "generated library `{}` has conflicting producers",
                        path.display()
                    )));
                }
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
    loop {
        let graph = graph::build(units)?;
        let rejected: Vec<_> = eligible
            .iter()
            .filter(|path| !owned(&claims[*path], units, &graph, catalog))
            .cloned()
            .collect();
        if rejected.is_empty() {
            for unit in units.values().filter_map(|unit| unit.as_ref().ok()) {
                if eligible.contains(&unit.source.path) && !unit.generated_library {
                    return Err(Diagnostic::error(format!(
                        "generated library output `{}` collides with an unmarked existing source",
                        unit.source.path.display()
                    )));
                }
            }
            return Ok(graph);
        }
        for path in rejected {
            eligible.remove(&path);
            for unit in units.values_mut() {
                if unit
                    .as_ref()
                    .is_ok_and(|unit| unit.source.path == path && unit.generated_library)
                {
                    *unit = Err(format!(
                        "generated library `{}` has no verified handwritten class producer `{}`",
                        path.display(),
                        claims[&path].producer.0
                    ));
                }
            }
        }
    }
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
        matches!(graph.scope(&claim.source).bind(&name, catalog), NamespaceBinding::Owned(owner) if owner == claim.producer)
    })
}
