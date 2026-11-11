use super::*;
use crate::namespace_sources::parser_cache::Cache;

/// Mandatory source snapshots and the independently loaded disposable parser facts.
pub(in crate::namespace_sources) struct ReadInputs {
    /// Complete UTF-8 sources selected by normal discovery.
    pub(in crate::namespace_sources) sources: BTreeMap<PathBuf, Arc<str>>,
    /// One immutable cache shared across the parser workers.
    pub(in crate::namespace_sources) parser_cache: Arc<Cache>,
}

/// Discovery errors retain priority over optional parser cache availability.
pub(in crate::namespace_sources) fn read_inputs(
    directory: &Path,
    cache_root: &Path,
    annotations: &[&str],
) -> Result<ReadInputs, Diagnostic> {
    std::thread::scope(|scope| {
        let cache = std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn_scoped(scope, || Cache::load(cache_root, annotations))
            .ok();
        let mut sources = BTreeMap::new();
        let read = collect_sources(directory, &mut sources);
        let cache = cache
            .map(|handle| handle.join().unwrap_or_default())
            .unwrap_or_default();
        read?;
        Ok(ReadInputs {
            sources,
            parser_cache: Arc::new(cache),
        })
    })
}
