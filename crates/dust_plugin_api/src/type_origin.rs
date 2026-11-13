use dust_ir::SpanIr;

/// A written type reference whose canonical declaration a plugin needs proven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeOriginRequest {
    /// The written type name, including an import prefix when present.
    pub name: String,
    /// The source span identifying this reference inside its library.
    pub span: SpanIr,
}
