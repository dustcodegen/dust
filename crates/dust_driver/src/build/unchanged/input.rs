use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use dust_cache::WorkspaceCache;
use dust_workspace::WorkspacePlan;

use crate::{
    build::{
        hash_text,
        process::OutputSnapshots,
        work::{available_worker_count, round_robin_groups},
    },
    result::BuildArtifact,
};

use super::manifest::{Artifact, Manifest};

/// Fresh emitter facts and optional prior verified output facts share validation.
pub(super) struct OutputEvidence<'a> {
    /// Digests retained from the current write-mode emitter results.
    pub(super) fresh: &'a OutputSnapshots,
    /// Prior proof retained only when the batch reused cached libraries.
    pub(super) previous: Option<&'a Manifest>,
}

/// Merges ordered file states, preserving the first path spelling and exact hashes.
pub(super) fn merge(
    left: BTreeMap<PathBuf, Option<u64>>,
    right: BTreeMap<PathBuf, Option<u64>>,
) -> Option<BTreeMap<PathBuf, Option<u64>>> {
    let mut files = Vec::with_capacity(left.len() + right.len());
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();
    while let (Some((left_path, _)), Some((right_path, _))) = (left.peek(), right.peek()) {
        match left_path.cmp(right_path) {
            std::cmp::Ordering::Less => files.push(left.next()?),
            std::cmp::Ordering::Greater => files.push(right.next()?),
            std::cmp::Ordering::Equal => {
                let entry = left.next()?;
                if entry.1 != right.next()?.1 {
                    return None;
                }
                files.push(entry);
            }
        }
    }
    files.extend(left);
    files.extend(right);
    Some(files.into_iter().collect())
}

/// Hashes the full UTF-8 contents, distinguishing absence from an empty file.
pub(super) fn read_hash(path: &Path) -> io::Result<Option<u64>> {
    read_text(path).map(|source| source.map(|source| hash_text(&source)))
}

/// Compares live cache bytes with the cache store's known-valid persisted digest.
pub(super) fn read_cache_hash(path: &Path) -> io::Result<Option<u64>> {
    read_text(path)
        .map(|source| source.map(|source| WorkspaceCache::hash_contents(source.as_bytes())))
}

/// Refuses devices and pipes before a disposable-proof read can block on them.
fn read_text(path: &Path) -> io::Result<Option<String>> {
    match fs::metadata(path) {
        Ok(metadata) if !metadata.is_file() => return Err(io::Error::other("not a regular file")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
        _ => {}
    }
    fs::read_to_string(path).map(Some)
}

/// Validates exact contents without retaining duplicate source text.
pub(super) fn matches(files: &BTreeMap<PathBuf, Option<u64>>) -> bool {
    let groups = round_robin_groups(files.iter(), available_worker_count(files.len(), None));
    std::thread::scope(|scope| {
        let handles = groups
            .into_iter()
            .map(|group| {
                scope.spawn(move || {
                    group.into_iter().all(|(path, expected)| {
                        read_hash(path).is_ok_and(|actual| actual == *expected)
                    })
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .all(|handle| handle.join().is_ok_and(|valid| valid))
    })
}

/// Uses the discovery walk's regular-file rule, retaining every Dart filename.
pub(super) fn lib_files(directory: &Path) -> Option<Vec<PathBuf>> {
    let mut files = Vec::new();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Some(files),
        Err(_) => return None,
    };
    let mut entries = entries.collect::<io::Result<Vec<_>>>().ok()?;
    entries.sort_by_cached_key(|entry| entry.file_name());
    for entry in entries {
        let kind = entry.file_type().ok()?;
        let path = entry.path();
        if kind.is_dir() {
            files.extend(lib_files(&path)?);
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "dart")
        {
            files.push(path);
        }
    }
    Some(files)
}

/// Excluded generated namespaces may be absent or retain the exact Dust banner.
pub(super) fn generated_hash(path: &Path) -> Option<Option<u64>> {
    match read_text(path).ok()? {
        Some(text) if crate::generated_source::is_dust_generated_source(&text) => {
            Some(Some(hash_text(&text)))
        }
        None => Some(None),
        _ => None,
    }
}

/// Verifies the expected ordered output sets before saving their exact file states.
pub(super) fn output_snapshot(
    workspace: &WorkspacePlan,
    cache: &WorkspaceCache,
    artifacts: &[BuildArtifact],
    tool: u64,
    config: u64,
    evidence: OutputEvidence<'_>,
) -> Option<BTreeMap<PathBuf, Option<u64>>> {
    let previous = evidence.previous;
    let prior_artifacts = previous
        .into_iter()
        .flat_map(|manifest| &manifest.artifacts)
        .map(|artifact| (artifact.0.as_path(), artifact))
        .collect::<BTreeMap<_, _>>();
    let groups = round_robin_groups(artifacts, available_worker_count(artifacts.len(), None));
    std::thread::scope(|scope| {
        let handles = groups
            .into_iter()
            .map(|group| {
                let prior_artifacts = &prior_artifacts;
                let fresh = evidence.fresh;
                scope.spawn(move || {
                    let mut files = BTreeMap::new();
                    for artifact in group {
                        let entry = cache.get(&workspace.cache_root, &artifact.source_path)?;
                        if entry.tool_hash != tool
                            || entry.package_config_hash != config
                            || entry
                                .analysis_snapshot
                                .string_set("dust_db_plugin.databases.v1")
                                .is_some()
                        {
                            return None;
                        }
                        if let Some(current) = fresh
                            .get(&artifact.source_path)
                            .and_then(|snapshot| snapshot.matching_files(artifact, entry))
                        {
                            files.extend(current.iter().cloned());
                            continue;
                        }
                        if let Some(reused) = previous.and_then(|manifest| {
                            reusable_outputs(
                                artifact,
                                entry.expected_output_hash,
                                prior_artifacts.get(artifact.source_path.as_path())?,
                                &manifest.files,
                            )
                        }) {
                            files.extend(reused);
                            continue;
                        }
                        let paths = std::iter::once(&artifact.output_path)
                            .chain(&artifact.auxiliary_output_paths);
                        let mut sources = Vec::new();
                        for (index, path) in paths.enumerate() {
                            let (source, hash) = match read_text(path).ok()? {
                                Some(source) => {
                                    let hash = hash_text(&source);
                                    (source, Some(hash))
                                }
                                // Analysis-only roots emit an empty primary; the expected hash still must match.
                                None if index == 0 => (String::new(), None),
                                _ => return None,
                            };
                            files.insert(path.clone(), hash);
                            sources.push((path, source));
                        }
                        if dust_emitter::hash_output_set(
                            sources
                                .iter()
                                .map(|(path, source)| (path.as_path(), source.as_str())),
                        ) != entry.expected_output_hash
                        {
                            return None;
                        }
                    }
                    Some(files)
                })
            })
            .collect::<Vec<_>>();
        let mut files = BTreeMap::new();
        for handle in handles {
            files.extend(handle.join().ok()??);
        }
        Some(files)
    })
}

/// An unchanged generated output set retains its already verified nonempty bytes.
fn reusable_outputs(
    artifact: &BuildArtifact,
    expected_hash: u64,
    previous: &Artifact,
    files: &BTreeMap<PathBuf, Option<u64>>,
) -> Option<BTreeMap<PathBuf, Option<u64>>> {
    if artifact.output_path != previous.1
        || artifact.auxiliary_output_paths != previous.2
        || expected_hash != previous.4
    {
        return None;
    }
    std::iter::once(&artifact.output_path)
        .chain(&artifact.auxiliary_output_paths)
        .map(|path| {
            let hash = files.get(path).copied().flatten()?;
            // Empty content and missing primaries share an output-set encoding.
            // Recheck their actual presence instead of inheriting that ambiguity.
            (hash != hash_text("")).then(|| (path.clone(), Some(hash)))
        })
        .collect()
}

/// Existing authoritative cache storage whose exact bytes invalidate the proof.
pub(super) fn cache_path(workspace: &WorkspacePlan) -> PathBuf {
    workspace
        .cache_root
        .join(".dart_tool/dust/build_cache_v1.json")
}

/// Runtime package versions are part of the proof even within a compatible range.
pub(super) fn package_specs(workspace: &WorkspacePlan) -> Option<BTreeMap<PathBuf, Option<u64>>> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&workspace.package_config.path).ok()?).ok()?;
    let mut files = BTreeMap::new();
    for package in value.get("packages")?.as_array()? {
        let name = package.get("name")?.as_str()?;
        if workspace.dust_packages.iter().any(|used| used == name) {
            let root = crate::source_uri::resolve_file_reference(
                &workspace.package_config.path,
                package.get("rootUri")?.as_str()?,
            )
            .ok()?;
            let path = root.join("pubspec.yaml");
            files.insert(path.clone(), read_hash(&path).ok()?);
        }
    }
    Some(files)
}

#[cfg(test)]
mod tests;
