#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = "Resolution helpers that map parsed Dust-relevant syntax into symbol-aware semantic data."]

/// Annotation lowering helpers.
mod annotations;
/// One scoped ownership policy for annotation applications and IR lowering.
mod binding;
/// Symbol catalog lookup.
mod catalog;
/// Resolver-owned database configuration normalization.
mod db;
/// Resolver-owned HTTP configuration normalization.
mod http;
/// Source-fact-driven annotation namespace binding.
mod namespace;
/// Resolved generated-part selection without URI re-parsing.
mod primary_part;
/// Resolver-owned DB query-call normalization.
mod query_calls;
/// Redirect constructor/type-path disambiguation using shared source facts.
mod redirects;
/// Library resolution entry points.
mod resolve;
/// Shared declaration resolution helpers.
mod resolve_support;
/// Resolved library result types.
mod result;
/// Resolver-owned route normalization.
mod route;
/// Resolver-owned SerDe normalization.
mod serde;
/// Resolver-owned state configuration normalization.
mod state;
/// Resolver-owned type lowering.
mod types;

pub use annotations::{
    annotation_ir_from_parsed, resolve_annotation_ir, resolve_annotation_ir_in_namespace,
};
pub use binding::BindingContext;
pub use catalog::{ResolvedSymbol, SymbolCatalog, SymbolKind};
pub use namespace::{NamespaceBinding, NamespaceFailureKind, NamespaceGraph, NamespaceScope};
pub use primary_part::PrimaryPart;
pub use resolve::{
    resolve_library, resolve_library_in_namespace, resolve_library_with_bindings,
    resolve_library_with_partless_configs, validate_generated_part_uri,
};
pub use result::{
    ResolveResult, ResolvedClass, ResolvedConstructor, ResolvedEnum, ResolvedEnumVariant,
    ResolvedField, ResolvedLibrary, ResolvedMethod, ResolvedMethodParam,
};
pub use types::{lower_type_ir, lower_type_ir_in_scope};
