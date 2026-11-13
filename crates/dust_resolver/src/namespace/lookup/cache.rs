use super::*;

/// A query stores only graph facts; ownership is recomputed for every catalog.
type Query = Result<Arc<Worlds>, NamespaceBinding>;
/// Concurrent same-name readers share one immutable query result.
type Slot = Arc<OnceLock<Query>>;

/// Prefix absence remains distinct from any explicit prefix spelling.
#[derive(Debug, Default)]
struct Prefixes {
    /// Ordinary unprefixed imports and local declarations.
    plain: BTreeMap<String, Slot>,
    /// Each declared or missing prefix owns an independent name namespace.
    qualified: BTreeMap<String, BTreeMap<String, Slot>>,
}

/// File IDs uniquely identify owning library URIs within one graph.
#[derive(Debug, Default)]
pub(in crate::namespace) struct LookupCache {
    /// Unrelated library queries need not select their slots under one global lock.
    entries: [Mutex<BTreeMap<FileId, Prefixes>>; 16],
}

impl LookupCache {
    /// Drops stale queries only after a successful graph insertion.
    pub(in crate::namespace) fn clear(&mut self) {
        for shard in &mut self.entries {
            shard
                .get_mut()
                .expect("lookup cache is not poisoned")
                .clear();
        }
    }

    /// Holds the map lock only while selecting a slot, never during namespace resolution.
    fn slot(&self, id: FileId, prefix: Option<&str>, name: &str) -> Slot {
        let shard = id.raw() as usize % self.entries.len();
        let mut entries = self.entries[shard]
            .lock()
            .expect("lookup cache is not poisoned");
        let library = entries.entry(id).or_default();
        let names = if let Some(prefix) = prefix {
            if !library.qualified.contains_key(prefix) {
                library.qualified.insert(prefix.to_owned(), BTreeMap::new());
            }
            library
                .qualified
                .get_mut(prefix)
                .expect("inserted lookup prefix")
        } else {
            &mut library.plain
        };
        if let Some(slot) = names.get(name) {
            Arc::clone(slot)
        } else {
            let slot = Arc::new(OnceLock::new());
            names.insert(name.to_owned(), Arc::clone(&slot));
            slot
        }
    }
}

impl NamespaceGraph {
    /// Parts share their owner namespace; lexical and constant decisions remain uncached.
    pub(in crate::namespace) fn lookup(
        &self,
        uri: &str,
        prefix: Option<&str>,
        name: &str,
    ) -> Result<Arc<Worlds>, NamespaceBinding> {
        let uri = self.owning_uri(uri)?;
        let slot = self.lookups.slot(self.units[uri].file_id, prefix, name);
        slot.get_or_init(|| self.compute_lookup(uri, prefix, name).map(Arc::new))
            .clone()
    }
}

#[cfg(test)]
mod tests;
