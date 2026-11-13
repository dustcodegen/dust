use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
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

/// Independent output planning preserves deterministic conflicts and errors.
mod plan;
pub(super) use plan::plan;

/// Ownership queries may need another import drain before outputs can be classified.
pub(super) enum Verification {
    /// Every query was resolved against imports already traversed by the loader.
    Ready(NamespaceGraph),
    /// Pending namespace requests must complete before any rejection in this round.
    NeedsImports(NamespaceGraph),
}

/// Removes unsupported placeholders only after each ownership round's imports are ready.
pub(super) fn verify_ready(
    units: &mut BTreeMap<String, Result<Unit, String>>,
    claims: &BTreeMap<PathBuf, Claim>,
    catalog: &SymbolCatalog,
    imports_ready: impl Fn(&BTreeSet<String>) -> bool,
) -> Result<Verification, Diagnostic> {
    let mut eligible: HashMap<_, _> = claims.iter().collect();
    let graph = loop {
        let mut graph = graph::build(units)?;
        let mut ownership = HashMap::new();
        let rejected: HashSet<_> = eligible
            .iter()
            .filter_map(|(&path, claim)| {
                (!ownership
                    .entry(&claim.source)
                    .or_insert_with(|| owners(&claim.source, units, &graph, catalog))
                    .contains(&claim.producer))
                .then_some(path)
            })
            .collect();
        if !imports_ready(&graph.required_lexical_imports()) {
            return Ok(Verification::NeedsImports(graph));
        }
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
        if !changed_units {
            // Keep cached namespaces, retaining requests only from surviving producers.
            if !graph.required_lexical_imports().is_empty() {
                graph.clear_required_lexical_imports();
                let sources = eligible
                    .values()
                    .map(|claim| claim.source.as_str())
                    .collect::<BTreeSet<_>>();
                for source in sources {
                    let _ = owners(source, units, &graph, catalog);
                }
            }
            break graph;
        }
    };
    for unit in units.values().filter_map(|unit| unit.as_ref().ok()) {
        if eligible.contains_key(&unit.source.path) && !unit.generated_library {
            return Err(Diagnostic::error(format!(
                "generated library output `{}` collides with an unmarked existing source",
                unit.source.path.display()
            )));
        }
    }
    Ok(Verification::Ready(graph))
}

/// Existing graph-only tests provide all required imports up front.
#[cfg(test)]
fn verify(
    units: &mut BTreeMap<String, Result<Unit, String>>,
    claims: &BTreeMap<PathBuf, Claim>,
    catalog: &SymbolCatalog,
) -> Result<NamespaceGraph, Diagnostic> {
    verify_ready(units, claims, catalog, |_| true).map(|verified| match verified {
        Verification::Ready(graph) | Verification::NeedsImports(graph) => graph,
    })
}

/// Only actual handwritten class metadata can authorize a declared output.
fn owners(
    source: &str,
    units: &BTreeMap<String, Result<Unit, String>>,
    graph: &NamespaceGraph,
    catalog: &SymbolCatalog,
) -> Vec<SymbolId> {
    let Some(Ok(unit)) = units.get(source) else {
        return Vec::new();
    };
    if unit.generated_library
        || !unit.namespace_diagnostics.is_empty()
        || unit
            .parsed
            .directives
            .iter()
            .any(|directive| matches!(directive, dust_parser_dart::ParsedDirective::PartOf { .. }))
    {
        return Vec::new();
    }
    unit.parsed
        .classes
        .iter()
        .flat_map(|class| &class.annotations)
        .filter_map(|annotation| {
            let name = annotation.prefix.as_ref().map_or_else(
                || annotation.name.clone(),
                |prefix| format!("{prefix}.{}", annotation.name),
            );
            match graph.scope(source).bind_at(
                &name,
                catalog,
                dust_ir::SpanIr::new(unit.file_id, annotation.span),
            ) {
                NamespaceBinding::Owned(owner) => Some(owner),
                _ => None,
            }
        })
        .collect()
}
