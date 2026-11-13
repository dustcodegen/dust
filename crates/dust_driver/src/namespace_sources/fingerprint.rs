use std::{cell::RefCell, path::Path};

use crate::build::{hash_text, update_hash_bytes};

use super::NamespaceSources;

/// URI ranks, adjacency and exact lazy record encodings.
mod index;
/// Reusable traversal memory local to each fingerprint worker.
mod scratch;

pub(super) use index::Index;
use scratch::Scratch;

thread_local! {
    /// Reuses traversal buffers without retaining identities between snapshots.
    static SCRATCH: RefCell<Scratch> = RefCell::default();
}

impl NamespaceSources {
    /// Hashes the unchanged v2 byte stream using sorted integer dependency ranks.
    pub(crate) fn fingerprint(&self, path: &Path) -> u64 {
        let mut hash = hash_text("dust-source-fingerprint-v2\0");
        update_hash_bytes(&mut hash, &self.metadata_hash.to_le_bytes());
        if let Some(root) = self.roots.get(path) {
            let index = self.fingerprint_index.get_or_init(|| Index::new(self));
            let root = index
                .uris
                .binary_search(root)
                .expect("indexed namespace root");
            SCRATCH.with(|scratch| {
                let mut scratch = scratch.borrow_mut();
                for &rank in scratch.reachable(index, root) {
                    update_hash_bytes(&mut hash, index.record(self, rank));
                }
            });
        }
        hash
    }
}

#[cfg(test)]
mod tests;
