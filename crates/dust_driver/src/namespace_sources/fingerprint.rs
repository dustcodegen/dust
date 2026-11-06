use std::path::Path;

use crate::build::{hash_text, update_hash_bytes};

use super::NamespaceSources;

impl NamespaceSources {
    /// Hashes an immutable source graph's metadata and sorted reachable records.
    pub(crate) fn fingerprint(&self, path: &Path) -> u64 {
        let mut hash = hash_text("dust-source-fingerprint-v2\0");
        update_hash_bytes(&mut hash, &self.metadata_hash.to_le_bytes());
        if let Some(root) = self.roots.get(path) {
            for uri in self.read_set(root) {
                hash_dependency(&mut hash, self, uri);
            }
        }
        hash
    }
}

/// Streams one length-delimited dependency record into the root fingerprint.
fn hash_dependency(hash: &mut u64, sources: &NamespaceSources, uri: &str) {
    hash_field(hash, uri);
    match sources.units.get(uri) {
        Some(Ok(unit)) => {
            update_hash_bytes(hash, b"\x01");
            update_hash_bytes(hash, &unit.file_id.raw().to_le_bytes());
            update_hash_bytes(hash, &(unit.text.len() as u64).to_le_bytes());
            let digest = unit.content_digest.get_or_init(|| hash_text(&unit.text));
            update_hash_bytes(hash, &digest.to_le_bytes());
            if let Some(owner) = &unit.generated_owner {
                update_hash_bytes(hash, b"\x01");
                hash_field(hash, owner);
            } else {
                update_hash_bytes(hash, b"\x00");
            }
        }
        Some(Err(error)) => {
            update_hash_bytes(hash, b"\x02");
            hash_field(hash, error);
        }
        None => update_hash_bytes(hash, b"\x03"),
    }
}

/// Hashes variable-width fields without depending on the host pointer width.
fn hash_field(hash: &mut u64, value: &str) {
    update_hash_bytes(hash, &(value.len() as u64).to_le_bytes());
    update_hash_bytes(hash, value.as_bytes());
}

#[cfg(test)]
mod tests;
