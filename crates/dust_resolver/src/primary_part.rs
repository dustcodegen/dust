//! Primary-part selection supplied by a caller with resolved source identities.

use dust_parser_dart::ParsedDirective;
use dust_text::TextRange;

/// Result of matching parsed parts against the configured physical output.
#[derive(Clone, Copy)]
pub enum PrimaryPart<'a> {
    /// No parsed part resolves to the configured output.
    Missing,
    /// The original declaration whose resolved target is the configured output.
    Resolved {
        /// Original parsed URI field, not a normalized path or replacement string.
        uri: &'a str,
        /// Original parsed directive span identifying the selected declaration.
        span: TextRange,
    },
}

impl PrimaryPart<'_> {
    /// Selects only the retained declaration; never substitutes its URI spelling.
    pub(crate) fn select<'a>(&self, directives: &'a [ParsedDirective]) -> Option<&'a str> {
        let Self::Resolved { uri, span } = self else {
            return None;
        };
        directives.iter().find_map(|directive| match directive {
            ParsedDirective::Part {
                uri: written,
                span: original,
                ..
            } if written == uri && original == span => Some(written.as_str()),
            _ => None,
        })
    }
}
