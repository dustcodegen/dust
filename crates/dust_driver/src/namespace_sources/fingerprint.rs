use std::{collections::HashMap, path::Path, sync::Arc};

use crate::build::{hash_text, update_hash_bytes};

use super::NamespaceSources;

/// Fingerprints roots within one immutable source snapshot.
pub(crate) struct NamespaceFingerprints<'a> {
    /// Source and dependency facts shared by all roots.
    sources: &'a NamespaceSources,
    /// Hash of the metadata record before any dependency bytes.
    seed: u64,
    /// Running hash after each exact, sorted SDK URI prefix.
    sdk_prefixes: HashMap<Vec<&'a str>, u64>,
}

impl<'a> NamespaceFingerprints<'a> {
    /// Starts a worker-local memo bound to this snapshot.
    pub(crate) fn new(sources: &'a NamespaceSources) -> Self {
        Self {
            sources,
            seed: hash_text(&format!("{}\0", sources.metadata_hash)),
            sdk_prefixes: HashMap::new(),
        }
    }

    /// Returns the retained root source and its transitive fingerprint.
    pub(crate) fn source_with_fingerprint(&mut self, path: &Path) -> Option<(Arc<str>, u64)> {
        let source = self.sources.source(path)?;
        Some((source, self.fingerprint(path)))
    }

    /// Hashes metadata, dependency texts, URI identities and diagnostic file IDs.
    pub(crate) fn fingerprint(&mut self, path: &Path) -> u64 {
        let sources = self.sources;
        let Some(root) = sources.roots.get(path) else {
            return self.seed;
        };
        let mut dependencies = sources.read_set(root).into_iter().peekable();
        let mut prefix = Vec::new();
        while let Some(uri) = dependencies.next_if(|uri| uri.starts_with("dart:")) {
            prefix.push(uri);
        }
        let mut hash = *self
            .sdk_prefixes
            .entry(prefix)
            .or_insert_with_key(|prefix| {
                let mut hash = self.seed;
                for uri in prefix {
                    hash_dependency(&mut hash, sources, uri);
                }
                hash
            });
        for uri in dependencies {
            hash_dependency(&mut hash, sources, uri);
        }
        hash
    }
}

/// Streams one dependency record in the existing fingerprint byte order.
fn hash_dependency(hash: &mut u64, sources: &NamespaceSources, uri: &str) {
    update_hash_bytes(hash, uri.as_bytes());
    update_hash_bytes(hash, b"\0");
    match sources.units.get(uri) {
        Some(Ok(unit)) => {
            update_hash_bytes(hash, unit.file_id.raw().to_string().as_bytes());
            update_hash_bytes(hash, b"\0");
            update_hash_bytes(hash, unit.text.as_bytes());
            if let Some(owner) = &unit.generated_owner {
                update_hash_bytes(hash, owner.as_bytes());
            }
        }
        Some(Err(error)) => update_hash_bytes(hash, error.as_bytes()),
        None => update_hash_bytes(hash, b"missing namespace source"),
    }
    update_hash_bytes(hash, b"\0");
}

/// Exact compatibility with the original concatenated fingerprint.
#[cfg(test)]
mod tests;
