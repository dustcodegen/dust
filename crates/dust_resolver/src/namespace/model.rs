use super::*;

/// A source declaration remains distinct through every re-export route.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Declaration {
    /// Owning library URI (rebased from a part to its owner).
    pub(super) uri: String,
    /// Declared name, not a canonical symbol's short spelling.
    pub(super) name: String,
    /// Actual defining unit's source file id.
    pub(super) file_id: FileId,
    /// Definition byte start.
    pub(super) start: u32,
    /// Definition byte end.
    pub(super) end: u32,
}

impl Declaration {
    /// A definition label does not lose the source location of a part.
    pub(super) fn label(&self) -> SourceLabel {
        SourceLabel::new(
            self.file_id,
            TextRange::new(self.start, self.end),
            format!("{} declares {}", self.uri, self.name),
        )
    }
}

/// Shape needed to distinguish declarations from genuinely opaque aliases.
#[derive(Debug, Clone)]
pub(super) enum DeclarationKind {
    /// Classes, enums and other definite non-alias declarations.
    Concrete,
    /// A parser-owned named type alias may forward to another declaration.
    Alias {
        /// The structured type name, including any import prefix.
        target: String,
        /// Scalar spelling is foreign only when no visible declaration shadows it.
        builtin: bool,
    },
    /// A known non-constructible alias cannot be a native annotation class.
    Foreign,
    /// Constant/value provenance is not available in existing parser facts.
    Opaque,
}

/// An unresolved edge is data, never a license for short-name fallback.
pub(super) type Target = Result<String, Problem>;

/// Source-attached problems have stable ordering in fixed-point alternatives.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Problem {
    /// Source unit of the failing directive.
    pub(super) file_id: FileId,
    /// Directive/literal byte start.
    pub(super) start: u32,
    /// Directive/literal byte end.
    pub(super) end: u32,
    /// The unresolved-input explanation.
    pub(super) message: String,
}

impl Problem {
    /// Captures original directive coordinates, not a synthetic whole-file span.
    pub(super) fn new(file_id: FileId, span: TextRange, message: String) -> Self {
        Self {
            file_id,
            start: span.start().to_u32(),
            end: span.end().to_u32(),
            message,
        }
    }
    /// Source labels are materialized only when a relevant name is used.
    pub(super) fn label(&self) -> SourceLabel {
        SourceLabel::new(
            self.file_id,
            TextRange::new(self.start, self.end),
            &self.message,
        )
    }
}

/// One import/export, including mutually exclusive URI alternatives.
#[derive(Debug)]
pub(super) struct Edge {
    /// Every default/conditional branch is retained, not host-selected.
    pub(super) targets: Vec<Target>,
    /// Ordered clauses from parser facts.
    pub(super) combinators: Vec<dust_parser_dart::ParsedVisibilityCombinator>,
    /// The written import prefix; exports are always unprefixed.
    pub(super) prefix: Option<String>,
    /// Deferred metadata must not become a normal owned annotation.
    pub(super) deferred: bool,
    /// Directive source location.
    pub(super) label: SourceLabel,
}

/// A `part of` header keeps URI and named-library forms distinct.
#[derive(Debug)]
pub(super) enum PartOwner {
    /// An explicit URI header, including any loader failure.
    Uri(Target, SourceLabel),
    /// A named header must match an actual owning library directive.
    Name(String, SourceLabel),
}

/// Only namespace-relevant parser facts are retained, not duplicate full ASTs.
#[derive(Debug)]
pub(super) struct Unit {
    /// Source file identity.
    pub(super) file_id: FileId,
    /// Optional Dart library name for named part headers.
    pub(super) library_name: Option<String>,
    /// Declaration identities and alias facts in this unit.
    pub(super) declarations: BTreeMap<Declaration, DeclarationKind>,
    /// Import namespaces are not exported implicitly.
    pub(super) imports: Vec<Edge>,
    /// Export dependencies are memoized separately from imports.
    pub(super) exports: Vec<Edge>,
    /// Parts contribute declarations in the owner's namespace.
    pub(super) parts: Vec<(Target, SourceLabel)>,
    /// A part must not be treated as an independent empty-import library.
    pub(super) part_of: Option<PartOwner>,
}

/// One possible namespace for a single name; alternatives stay mutually exclusive.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct World {
    /// Repeated paths to the same declaration deduplicate exactly.
    pub(super) declarations: BTreeSet<Declaration>,
    /// Required-input failures propagate without becoming foreign names.
    pub(super) problems: BTreeSet<Problem>,
}

/// Possible namespaces, not a union that invents coexisting conditional imports.
pub(super) type Worlds = BTreeSet<World>;
/// Export results for one name across all supplied library identities.
pub(super) type ExportWorlds = BTreeMap<String, Worlds>;
