use dust_diagnostics::{Diagnostic, render_to_string_with_files};
use dust_driver::CommandResult;

/// Renders diagnostics from one completed watch batch.
pub(crate) fn render_batch_diagnostics(batch: &CommandResult) -> String {
    let mut lines = Vec::new();
    append_diagnostic_blocks(&mut lines, batch, &batch.diagnostics);
    format!("{}\n", lines.join("\n"))
}

/// Appends rendered diagnostic blocks with source context.
pub(super) fn append_diagnostic_blocks(
    lines: &mut Vec<String>,
    result: &CommandResult,
    diagnostics: &[Diagnostic],
) {
    let files = result
        .diagnostic_files
        .iter()
        .map(|file| file.render_context())
        .collect::<Vec<_>>();
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        lines.extend(
            render_to_string_with_files(diagnostic, &files)
                .lines()
                .map(str::to_owned),
        );
        if index + 1 != diagnostics.len() {
            lines.push(String::new());
        }
    }
}
