use crate::{ExprSourceIr, SpanIr};

/// The visibility operation of one directive combinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombinatorKindIr {
    /// Retain only these names from the preceding visible set.
    Show,
    /// Remove these names from the preceding visible set.
    Hide,
}

/// One ordered visibility clause with its owning source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibilityCombinatorIr {
    /// The operation at this position.
    pub kind: CombinatorKindIr,
    /// Written names in source order.
    pub names: Vec<String>,
    /// The original whole-clause source span.
    pub span: SpanIr,
}

/// One conditional URI alternative; the condition is not evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalUriIr {
    /// Exact parenthesized condition source and span.
    pub condition: ExprSourceIr,
    /// Exact alternative URI literal source and span, not a decoded path.
    pub uri: ExprSourceIr,
    /// The full `if (...) 'uri'` source span.
    pub span: SpanIr,
}

/// Source-preserving namespace inputs for one import or export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceDirectiveIr {
    /// Exact default URI literal source and span, not a decoded path.
    pub default_uri: ExprSourceIr,
    /// Conditional alternatives in source order; no host branch is selected.
    pub conditional_uris: Vec<ConditionalUriIr>,
    /// Ordered visibility clauses; repeated `show` clauses stay separate.
    pub combinators: Vec<VisibilityCombinatorIr>,
}
