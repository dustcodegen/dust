use dust_workspace::SourceLibrary;

use crate::{
    build::process::{BuildOutcome, IndexedBuildOutcome},
    result::DiagnosticFile,
};

use super::{BatchConfig, BatchResult, ProgressReporter, ProgressSnapshot};

/// Ownership failures must not partially replace outputs or successful cache bytes.
pub(super) fn namespace_failure(
    config: BatchConfig<'_>,
    libraries: &[SourceLibrary],
    reporter: &ProgressReporter<'_>,
) -> Option<BatchResult> {
    if !libraries.iter().any(|library| {
        !config
            .namespaces
            .binding_errors(&library.source_path)
            .is_empty()
    }) {
        return None;
    }
    let mut outcomes = Vec::new();
    for (index, library) in libraries.iter().enumerate() {
        let diagnostics = config.namespaces.binding_errors(&library.source_path);
        if !diagnostics.is_empty() {
            outcomes.push(IndexedBuildOutcome {
                index,
                library: library.clone(),
                source_hash: None,
                tool_hash: None,
                outcome: BuildOutcome::failed(
                    library,
                    diagnostics.to_vec(),
                    Some(DiagnosticFile::new(
                        config
                            .namespaces
                            .file_id(&library.source_path)
                            .expect("namespace root id"),
                        library.source_path.clone(),
                        config
                            .namespaces
                            .source(&library.source_path)
                            .expect("namespace root text"),
                    )),
                ),
            });
        }
        reporter.finish(ProgressSnapshot {
            library,
            cached: false,
            routed: false,
            written: false,
            changed: false,
            had_errors: !diagnostics.is_empty(),
            elapsed_ms: 0,
        });
        if config.fail_fast && !outcomes.is_empty() {
            break;
        }
    }
    let diagnostic_files = super::dependency_diagnostic_files(config, &outcomes);
    Some(BatchResult {
        outcomes,
        workspace_analysis_hash: 0,
        diagnostic_files,
    })
}
