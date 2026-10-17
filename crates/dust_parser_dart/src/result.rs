use dust_diagnostics::Diagnostic;

use crate::{ParseOptions, ParsedDartFileSurface};

/// The backend-neutral result of parsing one Dart source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    /// The extracted library surface.
    pub library: ParsedDartFileSurface,
    /// Diagnostics emitted during parsing.
    pub diagnostics: Vec<Diagnostic>,
    /// Required interface failures, retaining syntax and language-version errors.
    ///
    /// A valid executable body may omit a generation-only limitation here.
    /// Generation callers must continue using `diagnostics` and `has_errors()`.
    pub namespace_diagnostics: Vec<Diagnostic>,
    /// The options used for this parse.
    pub options: ParseOptions,
}

impl ParseResult {
    /// Returns `true` if at least one diagnostic is present.
    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }
}
