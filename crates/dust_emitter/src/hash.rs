use std::path::Path;

use crate::EmitResult;

impl EmitResult {
    /// Stores the deterministic output-set hash for this emitted result.
    pub fn with_output_hash(mut self, output_path: &Path) -> Self {
        self.output_hash = Some(hash_output_set(
            std::iter::once((output_path, self.source.as_str())).chain(
                self.auxiliary_outputs
                    .iter()
                    .map(|output| (output.output_path.as_path(), output.source.as_str())),
            ),
        ));
        self
    }
}

/// Hashes the generated primary plus auxiliary output path/source pairs.
pub fn hash_output_set<'a>(outputs: impl IntoIterator<Item = (&'a Path, &'a str)>) -> u64 {
    let mut hash = 1469598103934665603_u64;
    for (path, source) in outputs {
        update_hash_bytes(&mut hash, path.to_string_lossy().as_bytes());
        update_hash_bytes(&mut hash, b"\0");
        update_hash_bytes(&mut hash, source.as_bytes());
        update_hash_bytes(&mut hash, b"\0");
    }
    hash
}

/// Updates a stable FNV-1a hash with raw bytes.
fn update_hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = (*hash).wrapping_mul(1099511628211);
    }
}
