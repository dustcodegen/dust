use dust_cache::WorkspaceCache;
use dust_text::FileId;

use crate::{build::flush_cache_into_result, result::CommandResult};

/// Persists a completed batch, reports it, and appends it to the watch result.
pub(super) fn finish_batch(
    mut batch: CommandResult,
    result: &mut CommandResult,
    last_file_id: &mut u32,
    cache: &mut WorkspaceCache,
    report: Option<&dyn Fn(&CommandResult)>,
) {
    flush_cache_into_result(cache, &mut batch);
    if let Some(report) = report {
        report(&batch);
    }
    append_batch(batch, result, last_file_id);
}

/// Combines results while keeping each diagnostic tied to its own source snapshot.
fn append_batch(mut batch: CommandResult, result: &mut CommandResult, last_file_id: &mut u32) {
    // Namespace snapshots start their file IDs at one on each reload.
    let previous_max = *last_file_id;
    result
        .diagnostic_files
        .extend(batch.diagnostic_files.into_iter().filter_map(|mut file| {
            file.file_id = rebase_id(file.file_id, previous_max, last_file_id)?;
            Some(file)
        }));
    for diagnostic in &mut batch.diagnostics {
        for label in &mut diagnostic.labels {
            // With no remaining IDs, rendering falls back to the label without a snippet.
            label.file_id =
                rebase_id(label.file_id, previous_max, last_file_id).unwrap_or_default();
        }
    }

    result.diagnostics.extend(batch.diagnostics);
    result.build_artifacts.extend(batch.build_artifacts);
}

/// Assigns a nonzero file ID after the previous batch's IDs.
fn rebase_id(file_id: FileId, previous_max: u32, last_file_id: &mut u32) -> Option<FileId> {
    if file_id.raw() == 0 {
        return None;
    }
    let id = file_id.raw().checked_add(previous_max)?;
    *last_file_id = (*last_file_id).max(id);
    Some(FileId::new(id))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use dust_diagnostics::{Diagnostic, SourceLabel};
    use dust_text::TextRange;

    use crate::result::DiagnosticFile;

    use super::*;

    #[test]
    fn collected_diagnostics_retain_the_source_for_each_batch() {
        let mut result = CommandResult::default();
        let mut last_file_id = 0;
        for source in ["first edit", "second edit"] {
            let id = FileId::new(1);
            let batch = CommandResult {
                diagnostics: vec![Diagnostic::error("broken").with_label(SourceLabel::new(
                    id,
                    TextRange::new(0_u32, 5_u32),
                    "bad token",
                ))],
                diagnostic_files: vec![DiagnosticFile::new(
                    id,
                    PathBuf::from("lib/user.dart"),
                    source,
                )],
                ..CommandResult::default()
            };
            append_batch(batch, &mut result, &mut last_file_id);
        }
        let sources = result
            .diagnostics
            .iter()
            .map(|diagnostic| {
                let id = diagnostic.labels[0].file_id;
                result
                    .diagnostic_files
                    .iter()
                    .find(|file| file.file_id == id)
                    .unwrap()
                    .source
                    .as_ref()
            })
            .collect::<Vec<_>>();
        assert_eq!(sources, ["first edit", "second edit"]);
    }
}
