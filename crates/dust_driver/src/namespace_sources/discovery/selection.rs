use super::*;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// Select roots independently and merge them in the original BTreeMap order.
pub(super) fn libraries(
    sources: &NamespaceSources,
    aware: &HashSet<String>,
    primary_suffix: &str,
    catalog: &SymbolCatalog,
    include_unannotated: bool,
) -> Vec<SourceLibrary> {
    let roots = sources.roots.iter().collect::<Vec<_>>();
    let workers = crate::build::available_worker_count(roots.len(), None);
    let next = AtomicUsize::new(0);
    let mut selected = std::thread::scope(|scope| {
        let handles = (0..workers)
            .map(|_| {
                let roots = &roots;
                let next = &next;
                scope.spawn(move || {
                    let mut selected = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Relaxed);
                        let Some(&(path, uri)) = roots.get(index) else {
                            break;
                        };
                        if let Some(library) = library(
                            sources,
                            path,
                            uri,
                            aware,
                            primary_suffix,
                            catalog,
                            include_unannotated,
                        ) {
                            selected.push((index, library));
                        }
                    }
                    selected
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("root selection must not panic"))
            .collect::<Vec<_>>()
    });
    selected.sort_by_key(|(index, _)| *index);
    selected.into_iter().map(|(_, library)| library).collect()
}

/// Keep the serial metadata, ownership and part-root predicates unchanged.
fn library(
    sources: &NamespaceSources,
    path: &Path,
    uri: &str,
    aware: &HashSet<String>,
    primary_suffix: &str,
    catalog: &SymbolCatalog,
    include_unannotated: bool,
) -> Option<SourceLibrary> {
    let unit = sources.units.get(uri)?.as_ref().ok()?;
    if unit
        .parsed
        .directives
        .iter()
        .any(|directive| matches!(directive, ParsedDirective::PartOf { .. }))
    {
        return None;
    }
    let metadata = metadata::annotations(&unit.parsed);
    let mut owned = false;
    let mut failed_input = false;
    for annotation in &metadata {
        let name = annotation.prefix.as_ref().map_or_else(
            || annotation.name.clone(),
            |prefix| format!("{prefix}.{}", annotation.name),
        );
        match sources.graph.scope(uri).bind_at(
            &name,
            catalog,
            dust_ir::SpanIr::new(unit.file_id, annotation.span),
        ) {
            NamespaceBinding::Owned(_) => owned = true,
            NamespaceBinding::Rejected { kind, .. }
                if kind != dust_resolver::NamespaceFailureKind::Hidden =>
            {
                failed_input = true;
            }
            _ => {}
        }
    }
    (owned
        || failed_input
        || (aware.contains(uri)
            && (!metadata.is_empty() || include_unannotated || !unit.diagnostics.is_empty())))
    .then(|| SourceLibrary {
        source_path: path.to_path_buf(),
        output_path: dust_workspace::primary_output_path(path, primary_suffix),
    })
}
