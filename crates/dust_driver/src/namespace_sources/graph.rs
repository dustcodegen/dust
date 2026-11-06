use std::collections::BTreeMap;

use dust_diagnostics::Diagnostic;
use dust_resolver::NamespaceGraph;

use super::loader::{Unit, implicit_core};

/// Builds a graph from retained facts and required interface diagnostics.
pub(super) fn build(
    units: &BTreeMap<String, Result<Unit, String>>,
) -> Result<NamespaceGraph, Diagnostic> {
    let mut graph = NamespaceGraph::new();
    for (uri, unit) in units {
        let Ok(unit) = unit else {
            continue;
        };
        let core = implicit_core(&unit.parsed.directives, uri);
        graph.insert_library_with_appended_directive(
            uri,
            unit.file_id,
            &unit.parsed,
            core.as_ref(),
            |_, reference| {
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
            },
        )?;
    }
    Ok(graph)
}
