use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use dust_diagnostics::Diagnostic;
use dust_workspace::WorkspacePlan;

use super::{NamespaceSources, index::SourceIndex};
use crate::build::hash_text;

impl NamespaceSources {
    /// Shares the exact text used for parsing and diagnostic ranges.
    pub(crate) fn source(&self, path: &Path) -> Option<Arc<str>> {
        Some(Arc::clone(&self.unit(path)?.text))
    }

    /// Only these paths were examined as generation roots during initial discovery.
    pub(crate) fn discovered_root_paths(&self) -> impl Iterator<Item = &Path> {
        self.roots.keys().map(PathBuf::as_path)
    }

    /// Retains original input bytes; generated placeholders are checked separately.
    pub(crate) fn input_snapshot(&self) -> Option<BTreeMap<PathBuf, u64>> {
        let mut inputs = self
            .metadata_inputs
            .iter()
            .map(|(path, digest)| (path.clone(), *digest))
            .collect::<Vec<_>>();
        for unit in self.units.values() {
            let unit = unit.as_ref().ok()?;
            if unit.generated_owner.is_some() || unit.generated_library {
                continue;
            }
            let digest = *unit.content_digest.get_or_init(|| hash_text(&unit.text));
            inputs.push((unit.source.path.clone(), digest));
        }
        inputs.sort_by(|(left, _), (right, _)| left.cmp(right));
        let mut conflict = false;
        inputs.dedup_by(|later, first| {
            if later.0 != first.0 {
                return false;
            }
            conflict |= later.1 != first.1;
            // Stable sorting retains the first key spelling, as BTreeMap::insert did.
            true
        });
        if conflict {
            return None;
        }
        Some(inputs.into_iter().collect())
    }

    /// Excluded source placeholders still require their ownership marker or absence.
    pub(crate) fn generated_paths(&self) -> BTreeSet<PathBuf> {
        self.units
            .values()
            .filter_map(|unit| unit.as_ref().ok())
            .filter(|unit| unit.generated_owner.is_some() || unit.generated_library)
            .map(|unit| unit.source.path.clone())
            .collect()
    }

    /// Resolves the current SDK identity and package metadata without parsing Dart.
    pub(crate) fn metadata_snapshot(
        cwd: &Path,
        workspace: &WorkspacePlan,
        sdk_root: Option<&Path>,
    ) -> Result<BTreeMap<PathBuf, u64>, Diagnostic> {
        let index = SourceIndex::load(
            cwd,
            &workspace.package_config.path,
            sdk_root,
            workspace.is_flutter_package,
        )
        .map_err(Diagnostic::error)?;
        Ok(index
            .inputs
            .into_iter()
            .map(|(path, text)| (path, hash_text(&text)))
            .collect())
    }
}

/// Bulk snapshots retain alias conflicts, metadata precedence and placeholder filtering.
#[cfg(test)]
mod tests;
