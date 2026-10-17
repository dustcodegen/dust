use std::collections::BTreeSet;

use crate::{build::process::IndexedBuildOutcome, result::DiagnosticFile};

use super::BatchConfig;

/// Retain referenced dependency files without duplicating root source contexts.
pub(super) fn dependency_diagnostic_files(
    config: BatchConfig<'_>,
    outcomes: &[IndexedBuildOutcome],
) -> Vec<DiagnosticFile> {
    let mut labels: BTreeSet<_> = outcomes
        .iter()
        .flat_map(|outcome| &outcome.outcome.diagnostics)
        .flat_map(|diagnostic| &diagnostic.labels)
        .map(|label| label.file_id)
        .collect();
    for outcome in outcomes {
        if let Some(file) = &outcome.outcome.diagnostic_file {
            labels.remove(&file.file_id);
        }
    }
    config
        .namespaces
        .diagnostic_files()
        .into_iter()
        .filter(|file| labels.contains(&file.file_id))
        .collect()
}
