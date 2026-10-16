use dust_text::TextRange;

use crate::ParsedTypeSurface;

/// One class type parameter with its original source locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTypeParamSurface {
    /// The written parameter name.
    pub name: String,
    /// Source span of the parameter name alone.
    pub name_span: TextRange,
    /// The optional written upper bound, without the `extends` keyword.
    pub bound: Option<ParsedTypeSurface>,
    /// Source span of the whole parameter, including metadata and bound.
    pub span: TextRange,
}
