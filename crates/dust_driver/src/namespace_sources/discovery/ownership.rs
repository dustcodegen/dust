use super::*;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// Validate independent selected libraries, then merge by deterministic source path.
pub(super) fn collect_errors(
    sources: &NamespaceSources,
    libraries: &[SourceLibrary],
    catalog: &SymbolCatalog,
    primary_suffix: &str,
) -> BTreeMap<PathBuf, Vec<Diagnostic>> {
    if libraries.is_empty() {
        return BTreeMap::new();
    }
    let workers = crate::build::available_worker_count(libraries.len(), None);
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let handles = (0..workers)
            .map(|_| {
                let next = &next;
                scope.spawn(move || {
                    let mut errors = Vec::new();
                    while let Some(library) = libraries.get(next.fetch_add(1, Relaxed)) {
                        if let Some(diagnostics) =
                            library_errors(sources, library, catalog, primary_suffix)
                        {
                            errors.push((library.source_path.clone(), diagnostics));
                        }
                    }
                    errors
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("ownership validation must not panic"))
            .collect()
    })
}

/// Dynamic worker assignment preserves selection and serial ownership diagnostics.
#[cfg(test)]
mod tests;

/// Keep metadata order and part guidance exactly as in serial validation.
fn library_errors(
    sources: &NamespaceSources,
    library: &SourceLibrary,
    catalog: &SymbolCatalog,
    primary_suffix: &str,
) -> Option<Vec<Diagnostic>> {
    let unit = sources.unit(&library.source_path).expect("selected source");
    if !unit.diagnostics.is_empty() {
        return None;
    }
    let bindings = dust_resolver::BindingContext::new(catalog, sources.scope(&library.source_path));
    let mut diagnostics = Vec::new();
    for annotation in metadata::annotations(&unit.parsed) {
        bindings.validate_annotation_ownership(unit.file_id, annotation, &mut diagnostics);
    }
    let part_note = unresolved_part_note(sources, library, unit, primary_suffix);
    let errors = diagnostics
        .into_iter()
        .filter(Diagnostic::is_error)
        .map(|diagnostic| match &part_note {
            Some(note) => diagnostic.with_note(note),
            None => diagnostic,
        })
        .collect::<Vec<_>>();
    (!errors.is_empty()).then_some(errors)
}

/// Explain the configured output without mistaking a missing foreign part for it.
fn unresolved_part_note(
    sources: &NamespaceSources,
    library: &SourceLibrary,
    unit: &loader::Unit,
    primary_suffix: &str,
) -> Option<String> {
    let mut has_expected = false;
    let mut has_failed = false;
    for directive in &unit.parsed.directives {
        let ParsedDirective::Part {
            uri_literal: Some(uri),
            ..
        } = directive
        else {
            continue;
        };
        let Some(reference) = uri.value() else {
            continue;
        };
        match unit.targets.get(&reference) {
            Some(Ok(target)) => {
                has_expected |= target.path == library.output_path;
                has_failed |= matches!(sources.units.get(&target.uri), Some(Err(_)));
            }
            Some(Err(_)) => has_failed = true,
            None => {}
        }
    }
    (has_failed && !has_expected).then(|| format!(
        "Dust's configured primary output for this library is `{}`; other parts must already exist",
        dust_workspace::expected_part_uri(&library.source_path, primary_suffix)
    ))
}
