use std::{collections::BTreeSet, sync::OnceLock};

use super::{NamespaceSources, hash_text};

/// One immutable source graph's ordered identities and reusable fingerprint facts.
pub(in crate::namespace_sources) struct Index {
    /// Lexical URI order is exactly the former BTreeSet traversal order.
    pub(super) uris: Vec<String>,
    /// Only successfully read units contribute outgoing dependency edges.
    pub(super) edges: Vec<Vec<usize>>,
    /// Reaching one root must not hash text belonging only to unrelated roots.
    records: Vec<OnceLock<Vec<u8>>>,
}

impl Index {
    /// Includes missing dependencies and root identities, not only loaded units.
    pub(super) fn new(sources: &NamespaceSources) -> Self {
        let mut uris = sources
            .units
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        uris.extend(sources.roots.values().map(String::as_str));
        for unit in sources.units.values().filter_map(|unit| unit.as_ref().ok()) {
            uris.extend(unit.dependencies.iter().map(String::as_str));
        }
        let uris = uris.into_iter().map(str::to_owned).collect::<Vec<_>>();
        let edges = uris
            .iter()
            .map(|uri| {
                sources
                    .units
                    .get(uri)
                    .and_then(|unit| unit.as_ref().ok())
                    .map(|unit| {
                        unit.dependencies
                            .iter()
                            .map(|dependency| {
                                uris.binary_search(dependency)
                                    .expect("indexed dependency URI")
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect();
        let records = (0..uris.len()).map(|_| OnceLock::new()).collect();
        Self {
            uris,
            edges,
            records,
        }
    }

    /// Uses the graph key as identity, even when a retained Unit has an alias key.
    pub(super) fn record(&self, sources: &NamespaceSources, rank: usize) -> &[u8] {
        self.records[rank].get_or_init(|| encode(sources, &self.uris[rank]))
    }
}

/// Encodes exactly the old hash_dependency byte sequence, with no extra digest layer.
fn encode(sources: &NamespaceSources, uri: &str) -> Vec<u8> {
    let mut record = Vec::new();
    field(&mut record, uri);
    match sources.units.get(uri) {
        Some(Ok(unit)) => {
            record.push(1);
            record.extend_from_slice(&unit.file_id.raw().to_le_bytes());
            record.extend_from_slice(&(unit.text.len() as u64).to_le_bytes());
            let digest = unit.content_digest.get_or_init(|| hash_text(&unit.text));
            record.extend_from_slice(&digest.to_le_bytes());
            if let Some(owner) = &unit.generated_owner {
                record.push(1);
                field(&mut record, owner);
            } else {
                record.push(0);
            }
        }
        Some(Err(error)) => {
            record.push(2);
            field(&mut record, error);
        }
        None => record.push(3),
    }
    record
}

/// Variable-width fields retain the exact u64 length and original UTF-8 bytes.
fn field(record: &mut Vec<u8>, value: &str) {
    record.extend_from_slice(&(value.len() as u64).to_le_bytes());
    record.extend_from_slice(value.as_bytes());
}
