use std::{collections::BTreeSet, sync::Arc};

use dust_text::FileId;

use crate::result::DiagnosticFile;

use super::NamespaceSources;

impl NamespaceSources {
    /// Supplies contexts for dependency labels without exposing generated contents.
    pub(crate) fn diagnostic_files(&self, labels: &BTreeSet<FileId>) -> Vec<DiagnosticFile> {
        if labels.is_empty() {
            return Vec::new();
        }
        self.units
            .values()
            .filter_map(|unit| unit.as_ref().ok())
            .filter(|unit| {
                labels.contains(&unit.file_id)
                    && unit.generated_owner.is_none()
                    && !unit.generated_library
            })
            .map(|unit| {
                DiagnosticFile::new(
                    unit.file_id,
                    unit.source.path.clone(),
                    Arc::clone(&unit.text),
                )
            })
            .collect()
    }
}
