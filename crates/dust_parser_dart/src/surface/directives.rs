use dust_text::TextRange;

/// One exact URI literal, not an evaluated or host-selected URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUriLiteral {
    /// Exact literal source, including raw/quoted delimiters and escapes.
    pub source: String,
    /// The original literal's UTF-8 byte range.
    pub span: TextRange,
}

impl ParsedUriLiteral {
    /// Decodes this single static literal without changing its source or span.
    pub fn value(&self) -> Option<String> {
        dust_dart_syntax::parse_static_dart_string_literal(&self.source)
    }
}

/// One conditional URI alternative in source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedConditionalUri {
    /// Exact condition source, including its parentheses and internal trivia.
    pub condition_source: String,
    /// The original condition's UTF-8 byte range.
    pub condition_span: TextRange,
    /// The alternative URI literal.
    pub uri: ParsedUriLiteral,
    /// The full `if (...) 'uri'` byte range.
    pub span: TextRange,
}

/// The visibility operation of one directive combinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParsedCombinatorKind {
    /// Retain only these names from the preceding visible set.
    Show,
    /// Remove these names from the preceding visible set.
    Hide,
}

/// One ordered `show` or `hide` clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedVisibilityCombinator {
    /// The operation to apply at this position.
    pub kind: ParsedCombinatorKind,
    /// Written names in source order, without flattening other clauses.
    pub names: Vec<String>,
    /// The original whole-clause UTF-8 byte range.
    pub span: TextRange,
}

/// Source-preserving namespace inputs for one import or export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNamespaceDirective {
    /// The unconditional default URI literal.
    pub default_uri: ParsedUriLiteral,
    /// Conditional alternatives in declaration order; no target is selected.
    pub conditional_uris: Vec<ParsedConditionalUri>,
    /// Visibility clauses in source order; repeated `show` is not a union.
    pub combinators: Vec<ParsedVisibilityCombinator>,
}
