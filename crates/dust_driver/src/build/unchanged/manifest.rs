use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use dust_cache::WorkspaceCache;
use serde::{Deserialize, Serialize};

use crate::result::BuildArtifact;

/// Source, ordered outputs, routing role and verified generated output-set hash.
pub(super) type Artifact = (PathBuf, PathBuf, Vec<PathBuf>, bool, u64);

/// Content-addressed proof of the last complete build, independent of parser objects.
#[derive(Serialize, Deserialize)]
pub(super) struct Manifest {
    /// Corruption check over every canonical payload field, excluding this digest.
    #[serde(skip_serializing)]
    pub(super) integrity: u64,
    /// Internal proof format; unsupported formats take the complete build path.
    pub(super) schema: u64,
    /// Codegen implementation and registry selection.
    pub(super) tool_hash: u64,
    /// Exact persistent cache bytes, including every per-library tool fingerprint.
    pub(super) cache_hash: u64,
    /// Active package/SDK metadata identities and content digests.
    pub(super) metadata: BTreeMap<PathBuf, u64>,
    /// Sorted discovered roots and verified present generated Dart paths.
    pub(super) lib_files: Vec<PathBuf>,
    /// Original input and verified output contents; None requires continued absence.
    pub(super) files: BTreeMap<PathBuf, Option<u64>>,
    /// Runtime packages whose current versions must still satisfy compatibility.
    pub(super) dust_packages: Vec<String>,
    /// Successful artifact summaries in deterministic source order.
    pub(super) artifacts: Vec<Artifact>,
}

impl Manifest {
    /// Malformed or inaccessible disposable proofs are cache misses.
    pub(super) fn load(root: &Path) -> Option<Self> {
        Self::decode(&read(root)?)
    }

    /// An untrusted header can reject a proof but never bypass complete validation.
    pub(super) fn load_matching(root: &Path, tool_hash: u64, cache_hash: u64) -> Option<Self> {
        let contents = read(root)?;
        let header: Header = serde_json::from_slice(&contents).ok()?;
        (header.schema == 3 && header.tool_hash == tool_hash && header.cache_hash == cache_hash)
            .then_some(())?;
        Self::decode(&contents)
    }

    /// Matching hints and history reads share the same complete payload checks.
    fn decode(contents: &[u8]) -> Option<Self> {
        let manifest: Self = serde_json::from_slice(contents).ok()?;
        (manifest.schema == 3).then_some(())?;
        let payload = serde_json::to_vec(&manifest).ok()?;
        (WorkspaceCache::hash_contents(&payload) == manifest.integrity).then_some(manifest)
    }

    /// This optional proof never changes authoritative cache or generated outputs.
    pub(super) fn save(self, root: &Path) {
        let Some(contents) = self.encode() else {
            return;
        };
        let proof_path = path(root);
        let Some(parent) = proof_path.parent() else {
            return;
        };
        let Ok(mut temporary) = tempfile::Builder::new()
            .prefix(".unchanged_build.")
            .tempfile_in(parent)
        else {
            return;
        };
        if temporary.write_all(&contents).is_ok() {
            // Replace the cache entry itself, including a symlink, never its target.
            let _ = temporary.persist(proof_path);
        }
    }
    /// Serialization failures, including non-UTF-8 paths, disable the optional proof.
    fn encode(self) -> Option<Vec<u8>> {
        let payload = serde_json::to_vec(&self).ok()?;
        let integrity = WorkspaceCache::hash_contents(&payload);
        let mut contents = Vec::with_capacity(payload.len().checked_add(40)?);
        write!(&mut contents, "{{\"integrity\":{integrity},").ok()?;
        // Reuse the exact canonical object instead of serializing all paths and maps again.
        contents.extend_from_slice(payload.strip_prefix(b"{")?);
        Some(contents)
    }
}

/// Serde skips all remaining payload fields without allocating their maps or vectors.
#[derive(Deserialize)]
struct Header {
    /// Unsupported schemas cannot provide a matching proof.
    schema: u64,
    /// Generation implementation expected by the proof.
    tool_hash: u64,
    /// Exact authoritative cache bytes expected by the proof.
    cache_hash: u64,
}

#[cfg(test)]
mod tests;

/// The optional proof is removed by the existing Dust cache clean operation.
fn path(root: &Path) -> PathBuf {
    root.join(".dart_tool/dust/unchanged_build_v1.json")
}

/// Optional proof reads must not block on a nonregular filesystem entry.
fn read(root: &Path) -> Option<Vec<u8>> {
    let proof_path = path(root);
    fs::metadata(&proof_path).ok()?.is_file().then_some(())?;
    fs::read(proof_path).ok()
}

/// Reused outputs have no writes or pending changes.
pub(super) fn artifact(
    (source_path, output_path, auxiliary_output_paths, routed, _): Artifact,
) -> BuildArtifact {
    BuildArtifact {
        source_path,
        output_path,
        auxiliary_output_paths,
        routed,
        changed: false,
        written: false,
        cached: true,
    }
}
