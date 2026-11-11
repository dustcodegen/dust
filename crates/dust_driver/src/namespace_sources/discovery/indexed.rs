use super::*;
use crate::namespace_sources::index::{Source, SourceIndex};

/// Resolves independent source identities and preserves the original first error.
pub(in crate::namespace_sources) fn index_sources(
    index: &SourceIndex,
    files: &BTreeMap<PathBuf, Arc<str>>,
) -> Result<Vec<Source>, Diagnostic> {
    let groups = crate::build::round_robin_groups(
        files.keys().enumerate(),
        crate::build::available_worker_count(files.len(), None),
    );
    let mut indexed = std::thread::scope(|scope| {
        let handles = groups
            .into_iter()
            .map(|group| {
                scope.spawn(move || {
                    group
                        .into_iter()
                        .map(|(order, path)| (order, index.file(path)))
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("source indexing must not panic"))
            .collect::<Vec<_>>()
    });
    indexed.sort_by_key(|(order, _)| *order);
    indexed
        .into_iter()
        .map(|(_, source)| source.map_err(Diagnostic::error))
        .collect()
}
