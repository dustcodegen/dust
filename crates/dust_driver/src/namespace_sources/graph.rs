use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

use dust_diagnostics::Diagnostic;
use dust_resolver::{NamespaceGraph, PreparedNamespaceLibrary};

use super::loader::{Unit, implicit_core};

/// Builds a graph from retained facts and required interface diagnostics.
pub(super) fn build(
    units: &BTreeMap<String, Result<Unit, String>>,
) -> Result<NamespaceGraph, Diagnostic> {
    let inputs = units
        .iter()
        .enumerate()
        .filter_map(|(index, (uri, unit))| Some((index, uri.as_str(), unit.as_ref().ok()?)))
        .collect::<Vec<_>>();
    let next = AtomicUsize::new(0);
    let prepare = || prepare_entries(units, &inputs, &next);
    let mut prepared = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        let mut prepared = Vec::new();
        let mut fallback = false;
        for _ in 0..crate::build::available_worker_count(inputs.len(), None) {
            // Owning lexical facts retain the parser workers' recursive syntax budget.
            match std::thread::Builder::new()
                .stack_size(8 * 1024 * 1024)
                .spawn_scoped(scope, prepare)
            {
                Ok(handle) => handles.push(handle),
                Err(_) => fallback = true,
            }
        }
        if fallback {
            prepared.extend(prepare());
        }
        for handle in handles {
            prepared.extend(handle.join().expect("namespace preparation must not panic"));
        }
        prepared
    });
    prepared.sort_unstable_by_key(|(index, _)| *index);
    let mut graph = NamespaceGraph::new();
    for (_, library) in prepared {
        graph.insert_prepared_library(library)?;
    }
    Ok(graph)
}

/// Normalization reads immutable source facts and preserves each directive's order.
fn prepare_entries(
    units: &BTreeMap<String, Result<Unit, String>>,
    inputs: &[(usize, &str, &Unit)],
    next: &AtomicUsize,
) -> Vec<(usize, PreparedNamespaceLibrary)> {
    let mut prepared = Vec::new();
    while let Some(&(index, uri, unit)) = inputs.get(next.fetch_add(1, Relaxed)) {
        let core = implicit_core(&unit.parsed.directives, uri);
        let library = NamespaceGraph::prepare_library(
            uri,
            unit.file_id,
            &unit.parsed,
            core.as_ref(),
            |_, reference| resolve(units, unit, reference),
        );
        prepared.push((index, library));
    }
    prepared
}

/// Source read failures and interface diagnostics remain attached to their edges.
fn resolve(
    units: &BTreeMap<String, Result<Unit, String>>,
    unit: &Unit,
    reference: &str,
) -> Result<String, String> {
    let source = unit
        .targets
        .get(reference)
        .ok_or_else(|| format!("missing URI facts for `{reference}`"))?
        .as_ref()
        .map_err(Clone::clone)?;
    match units.get(&source.uri) {
        Some(Err(error)) => Err(error.clone()),
        Some(Ok(target)) if !target.namespace_diagnostics.is_empty() => Err(format!(
            "cannot parse namespace source `{}`: {}",
            target.source.path.display(),
            target
                .namespace_diagnostics
                .iter()
                .map(|error| error.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        )),
        _ => Ok(source.uri.clone()),
    }
}

/// Parallel preparation retains serial graph diagnostics and ownership.
#[cfg(test)]
mod tests;
