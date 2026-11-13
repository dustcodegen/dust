use super::*;
use dust_parser_dart::ParsedDirective;
use dust_workspace::SourceLibrary;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// Parallel source identities in discovery order.
mod indexed;
pub(super) use indexed::index_sources;
/// Optional parser snapshots load alongside mandatory discovery reads.
mod inputs;
pub(super) use inputs::read_inputs;

/// Complete metadata ownership validation before output mutation.
mod ownership;
/// Parallel root selection with deterministic library order.
mod selection;

#[cfg(test)]
mod tests;

/// Selects Dust-aware roots using bound metadata and retains ownership failures.
pub(super) fn select_libraries(
    sources: &mut NamespaceSources,
    workspace: &mut WorkspacePlan,
    catalog: &SymbolCatalog,
    include_unannotated: bool,
) -> HashSet<String> {
    let aware = dust_aware_uris(sources, catalog)
        .into_iter()
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    workspace.libraries = selection::libraries(
        sources,
        &aware,
        &workspace.dust_config.outputs.primary_suffix,
        catalog,
        include_unannotated,
    );
    workspace.dust_packages = dust_workspace::dust_runtime_packages()
        .into_iter()
        .filter(|name| {
            let prefix = format!("package:{name}/");
            sources.units.keys().any(|uri| uri.starts_with(&prefix))
        })
        .map(str::to_owned)
        .collect();
    let errors = ownership::collect_errors(
        sources,
        &workspace.libraries,
        catalog,
        &workspace.dust_config.outputs.primary_suffix,
    );
    sources.binding_errors.extend(errors);
    aware
}

/// Finds every URI that can reach an annotation origin through traversed edges.
pub(super) fn dust_aware_uris<'a>(
    sources: &'a NamespaceSources,
    catalog: &SymbolCatalog,
) -> HashSet<&'a str> {
    let mut dependents = HashMap::<&str, Vec<&str>>::new();
    for (uri, unit) in &sources.units {
        if let Ok(unit) = unit {
            for dependency in &unit.dependencies {
                dependents.entry(dependency).or_default().push(uri);
            }
        }
    }
    // A referenced origin may be missing or unreadable and still count for discovery.
    let mut queue: Vec<_> = catalog
        .annotation_origins()
        .map(|origin| origin.library_uri)
        .collect();
    let mut aware = HashSet::new();
    while let Some(uri) = queue.pop() {
        if aware.insert(uri)
            && let Some(dependents) = dependents.get(uri)
        {
            queue.extend(dependents.iter().copied());
        }
    }
    aware
}

/// Finds handwritten or foreign-generated Dart sources, excluding marked Dust output.
pub(super) fn collect_sources(
    directory: &Path,
    sources: &mut BTreeMap<PathBuf, Arc<str>>,
) -> Result<(), Diagnostic> {
    let mut paths = Vec::new();
    collect_paths(directory, &mut paths)?;
    sort_paths(directory, &mut paths);
    let workers = crate::build::available_worker_count(paths.len(), None);
    let next = AtomicUsize::new(0);
    let read = || {
        let mut selected = Vec::new();
        loop {
            let order = next.fetch_add(1, Relaxed);
            let Some(path) = paths.get(order) else {
                return selected;
            };
            let input = fs::read_to_string(path)
                .map(|text| {
                    if crate::generated_source::is_dust_generated_source(&text) {
                        None
                    } else {
                        Some((path.clone(), Arc::<str>::from(text)))
                    }
                })
                .map_err(|error| {
                    Diagnostic::error(format!("cannot read `{}`: {error}", path.display()))
                });
            selected.push((order, input));
        }
    };
    let mut selected = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        let mut selected = Vec::new();
        let mut fallback = false;
        let read = &read;
        for _ in 0..workers {
            match std::thread::Builder::new().spawn_scoped(scope, read) {
                Ok(handle) => handles.push(handle),
                Err(_) => fallback = true,
            }
        }
        if fallback {
            // Failed spawns reserve no indices; the coordinator shares all remaining work.
            selected.extend(read());
        }
        for handle in handles {
            selected.extend(handle.join().expect("source scan must not panic"));
        }
        selected
    });
    selected.sort_by_key(|(index, _)| *index);
    let mut discovered = Vec::new();
    for (_, input) in selected {
        match input {
            Ok(Some(source)) => discovered.push(source),
            Ok(None) => {}
            Err(error) => {
                sources.extend(discovered);
                return Err(error);
            }
        }
    }
    if sources.is_empty() {
        *sources = discovered.into_iter().collect();
    } else {
        // Insert semantics retain an existing equal path's original spelling.
        sources.extend(discovered);
    }
    Ok(())
}

/// One shared prefix cannot affect ordering; cache short keys instead of rescanning it.
fn sort_paths(directory: &Path, paths: &mut [PathBuf]) {
    paths.sort_by_cached_key(|path| {
        path.strip_prefix(directory)
            .expect("collected source paths share their discovery directory")
            .to_path_buf()
    });
}

/// Collects paths before reading independent source files in parallel.
fn collect_paths(directory: &Path, sources: &mut Vec<PathBuf>) -> Result<(), Diagnostic> {
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
            collect_paths(&path, sources)?;
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "dart")
        {
            sources.push(path);
        }
    }
    Ok(())
}
