use super::*;
use crate::build::update_hash_bytes;

/// The exact former v2 implementation, kept separate from record encoding.
pub(super) fn fingerprint(sources: &NamespaceSources, path: &Path) -> u64 {
    let mut hash = hash_text("dust-source-fingerprint-v2\0");
    update_hash_bytes(&mut hash, &sources.metadata_hash.to_le_bytes());
    if let Some(root) = sources.roots.get(path) {
        for uri in sources.read_set(root) {
            field(&mut hash, uri);
            match sources.units.get(uri) {
                Some(Ok(unit)) => {
                    update_hash_bytes(&mut hash, b"\x01");
                    update_hash_bytes(&mut hash, &unit.file_id.raw().to_le_bytes());
                    update_hash_bytes(&mut hash, &(unit.text.len() as u64).to_le_bytes());
                    let digest = unit.content_digest.get_or_init(|| hash_text(&unit.text));
                    update_hash_bytes(&mut hash, &digest.to_le_bytes());
                    if let Some(owner) = &unit.generated_owner {
                        update_hash_bytes(&mut hash, b"\x01");
                        field(&mut hash, owner);
                    } else {
                        update_hash_bytes(&mut hash, b"\x00");
                    }
                }
                Some(Err(error)) => {
                    update_hash_bytes(&mut hash, b"\x02");
                    field(&mut hash, error);
                }
                None => update_hash_bytes(&mut hash, b"\x03"),
            }
        }
    }
    hash
}

/// Original field framing hashes byte lengths, not character counts.
fn field(hash: &mut u64, value: &str) {
    update_hash_bytes(hash, &(value.len() as u64).to_le_bytes());
    update_hash_bytes(hash, value.as_bytes());
}
