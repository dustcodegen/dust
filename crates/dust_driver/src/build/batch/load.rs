use dust_diagnostics::Diagnostic;
use dust_workspace::SourceLibrary;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::build::{
    batch::BatchConfig,
    process::LoadedLibraryInput,
    support::{CacheFingerprint, load_library_input},
    work::available_worker_count,
};

/// Loads source text and prior output fingerprints for all libraries.
pub(super) fn load_library_inputs(
    config: BatchConfig<'_>,
    libraries: &[SourceLibrary],
) -> Vec<Result<LoadedLibraryInput, Diagnostic>> {
    let threads = available_worker_count(libraries.len(), None);
    let next = AtomicUsize::new(0);

    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(threads);
        for _ in 0..threads {
            let next = &next;
            handles.push(scope.spawn(move || {
                let mut loaded = Vec::new();
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(library) = libraries.get(index) else {
                        break;
                    };
                    let cache_fingerprint = config
                        .cache
                        .get(config.cache_root, &library.source_path)
                        .filter(|entry| {
                            dust_db_plugin::database_migration_cache_matches(
                                config.package_root,
                                &entry.analysis_snapshot,
                            )
                        })
                        .map(|entry| CacheFingerprint {
                            source_hash: entry.source_hash,
                            package_config_hash: entry.package_config_hash,
                            tool_hash: entry.tool_hash,
                            output_paths: std::iter::once(library.output_path.clone())
                                .chain(entry.auxiliary_output_paths.iter().cloned())
                                .collect(),
                        });
                    let input = load_library_input(
                        library,
                        cache_fingerprint,
                        config.package_config_hash,
                        config.tool_hash,
                        config.namespaces,
                    );
                    loaded.push((index, input));
                }
                loaded
            }));
        }

        let mut ordered = (0..libraries.len()).map(|_| None).collect::<Vec<_>>();
        for handle in handles {
            for (index, input) in handle.join().expect("load thread must not panic") {
                ordered[index] = Some(input);
            }
        }

        ordered
            .into_iter()
            .map(|entry| entry.expect("every library load slot must be filled"))
            .collect()
    })
}
