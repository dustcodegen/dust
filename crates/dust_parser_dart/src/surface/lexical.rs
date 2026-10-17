//! Source-owned lexical names and constant provenance, separate from generation fields.

use dust_text::TextRange;

use super::ParsedAnnotationArguments;

/// One identifier in a lexical declaration or a constant reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLexicalName {
    /// Semantic identifier spelling, excluding qualifier trivia.
    pub name: String,
    /// Exact identifier coordinates in the original file.
    pub span: TextRange,
}

/// One scope relevant to represented metadata uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLexicalScope {
    /// Parent index in the same file's ordered scope vector.
    pub parent: Option<usize>,
    /// Named owner when the source has one; the library has no owner name.
    pub name: Option<ParsedLexicalName>,
    /// Whether this is a library, type or callable namespace.
    pub kind: ParsedLexicalScopeKind,
    /// Region where this scope participates in lookup, excluding outer metadata.
    pub visibility: TextRange,
    /// Whole source declaration, including its outer metadata when present.
    pub span: TextRange,
    /// Direct lexical declarations; static members are not instance-field facts.
    pub declarations: Vec<ParsedLexicalDeclaration>,
}

/// Lexical scope shape, without claiming full Dart type checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParsedLexicalScopeKind {
    /// File-level constants and the parent of declaration scopes.
    Library,
    /// A class, mixin, enum, extension or extension type.
    Type,
    /// A callable's own generic binders, not its parameter metadata's value names.
    Callable,
}

/// One declared name which can shadow an imported spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLexicalDeclaration {
    /// Name and exact declaration identifier span.
    pub name: ParsedLexicalName,
    /// Whether constant provenance can be inspected or the name is only a shadow.
    pub kind: ParsedLexicalDeclarationKind,
    /// Original declaration coordinates, never synthesized source.
    pub span: TextRange,
}

/// Minimal declaration facts needed before annotation ownership is selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedLexicalDeclarationKind {
    /// A written constant with its original initializer, if present.
    Constant(Option<ParsedLexicalInitializer>),
    /// A generic binder shadows values even though it is not an annotation value.
    TypeParameter,
    /// An enum's implicitly constant value, without inventing initializer source.
    EnumValue,
    /// A non-constant member; no imported-owner fallback is justified.
    Member,
}

/// A constant initializer retains both exact source and structural provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLexicalInitializer {
    /// Exact written expression, without surrounding declaration punctuation.
    pub source: String,
    /// Original expression coordinates.
    pub span: TextRange,
    /// Supported identity facts; unsupported expressions stay explicitly opaque.
    pub kind: ParsedLexicalInitializerKind,
}

/// Structured identity, not evaluated Dart constant values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedLexicalInitializerKind {
    /// A direct invocation, retaining actual argument values for later normalization.
    Constructor {
        /// Callee identifiers only, excluding type arguments and comments.
        callee: Vec<ParsedLexicalName>,
        /// Existing parser-owned argument facts, never reparsed source strings.
        arguments: ParsedAnnotationArguments,
    },
    /// A direct constant reference with semantic identifiers and original spans.
    Reference(Vec<ParsedLexicalName>),
    /// No ownership may be inferred from this expression's source spelling.
    Opaque,
}
