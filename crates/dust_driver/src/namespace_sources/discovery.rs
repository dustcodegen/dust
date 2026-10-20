use super::*;
use dust_parser_dart::ParsedDirective;
use dust_workspace::SourceLibrary;
use std::fs;

/// Selects Dust-aware roots using bound metadata and retains ownership failures.
pub(super) fn select_libraries(
    sources: &mut NamespaceSources,
    workspace: &mut WorkspacePlan,
    catalog: &SymbolCatalog,
    include_unannotated: bool,
) {
    workspace.libraries = sources
        .roots
        .iter()
        .filter_map(|(path, uri)| {
            let unit = sources.unit(path)?;
            if unit
                .parsed
                .directives
                .iter()
                .any(|directive| matches!(directive, ParsedDirective::PartOf { .. }))
            {
                return None;
            }
            let read_set = sources.read_set(uri);
            let aware = catalog
                .annotation_origins()
                .any(|origin| read_set.contains(origin.library_uri));
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
                || (aware
                    && (!metadata.is_empty()
                        || include_unannotated
                        || !unit.diagnostics.is_empty())))
            .then(|| SourceLibrary {
                source_path: path.clone(),
                output_path: dust_workspace::primary_output_path(
                    path,
                    &workspace.dust_config.outputs.primary_suffix,
                ),
            })
        })
        .collect();
    workspace.dust_packages = dust_workspace::dust_runtime_packages()
        .into_iter()
        .filter(|name| {
            sources
                .units
                .keys()
                .any(|uri| uri.starts_with(&format!("package:{name}/")))
        })
        .map(str::to_owned)
        .collect();
    for library in &workspace.libraries {
        let unit = sources.unit(&library.source_path).expect("selected source");
        if !unit.diagnostics.is_empty() {
            continue;
        }
        let bindings =
            dust_resolver::BindingContext::new(catalog, sources.scope(&library.source_path));
        let mut diagnostics = Vec::new();
        for annotation in metadata::annotations(&unit.parsed) {
            let _ = bindings.annotation_ir(unit.file_id, annotation, &mut diagnostics);
        }
        let part_note = unresolved_part_note(
            sources,
            library,
            unit,
            &workspace.dust_config.outputs.primary_suffix,
        );
        let errors = diagnostics
            .into_iter()
            .filter(Diagnostic::is_error)
            .map(|diagnostic| match &part_note {
                Some(note) => diagnostic.with_note(note),
                None => diagnostic,
            })
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            sources
                .binding_errors
                .insert(library.source_path.clone(), errors);
        }
    }
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

/// Finds handwritten or foreign-generated Dart sources, excluding marked Dust output.
pub(super) fn collect_sources(
    directory: &Path,
    sources: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    if !directory.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(directory).map_err(|error| {
        Diagnostic::error(format!("cannot read `{}`: {error}", directory.display()))
    })? {
        let entry = entry.map_err(|error| Diagnostic::error(error.to_string()))?;
        let kind = entry
            .file_type()
            .map_err(|error| Diagnostic::error(error.to_string()))?;
        let path = entry.path();
        if kind.is_dir() {
            collect_sources(&path, sources)?;
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "dart")
        {
            let text = fs::read_to_string(&path).map_err(|error| {
                Diagnostic::error(format!("cannot read `{}`: {error}", path.display()))
            })?;
            if !crate::generated_source::is_dust_generated_source(&text) {
                sources.push(path);
            }
        }
    }
    Ok(())
}
