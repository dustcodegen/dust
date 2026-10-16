use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::{NameIr, SpanIr, SymbolId};
use dust_parser_dart::ParsedAnnotation;
use dust_text::FileId;

use crate::{NamespaceBinding, NamespaceScope, ResolvedSymbol, SymbolCatalog};

/// Shared ownership decision before selecting a semantic role or normalizing.
pub(crate) struct BindingContext<'a> {
    /// Effective canonical symbols and declaration origins.
    pub(crate) catalog: &'a SymbolCatalog,
    /// Absent only for the explicit context-free compatibility adapter.
    pub(crate) namespace: Option<NamespaceScope<'a>>,
}

/// The intrinsic grouping marker is not a plugin trait/config claim.
pub(crate) enum Application<'a> {
    /// The intrinsic group, before resolving its nested trait values.
    Group,
    /// One bound owner with its selected semantic role.
    Symbol(&'a ResolvedSymbol),
    /// Foreign, unsupported or diagnosed metadata has no application.
    Unowned,
}

impl<'a> BindingContext<'a> {
    /// Raw parsed names and spans are never rewritten to canonical strings.
    pub(crate) fn application(
        &self,
        file_id: FileId,
        annotation: &ParsedAnnotation,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Application<'a> {
        if self.namespace.is_none() {
            if annotation.is_named("Derive") {
                return Application::Group;
            }
            return self
                .catalog
                .resolve_qualified_config(&annotation.qualified_name)
                .or_else(|| self.catalog.resolve_config(&annotation.name))
                .or_else(|| {
                    self.catalog
                        .resolve_qualified_trait(&annotation.qualified_name)
                })
                .or_else(|| self.catalog.resolve_trait(&annotation.name))
                .map_or(Application::Unowned, Application::Symbol);
        }
        let owner = self.owner(
            &annotation.qualified_name,
            &annotation.name,
            SpanIr::new(file_id, annotation.span),
            false,
            diagnostics,
        );
        match owner.as_ref().map(|id| id.0.as_str()) {
            Some("dust_dart::Derive") => Application::Group,
            Some(id) => self
                .catalog
                .resolve_qualified(id)
                .map_or(Application::Unowned, Application::Symbol),
            None => Application::Unowned,
        }
    }

    /// Nested constructor values retain their written prefix and original span.
    pub(crate) fn derive_trait(
        &self,
        name: &NameIr,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<&'a ResolvedSymbol> {
        if self.namespace.is_none() {
            return self.catalog.resolve_trait(&name.short);
        }
        let owner = self.owner(&name.source, &name.short, name.span, true, diagnostics)?;
        self.catalog.resolve_qualified_trait(&owner.0)
    }

    /// Unknown provenance alone permits unique-owner legacy compatibility.
    pub(crate) fn owner(
        &self,
        written: &str,
        short: &str,
        span: SpanIr,
        trait_only: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SymbolId> {
        let binding = self.namespace.map_or(NamespaceBinding::Unknown, |scope| {
            scope.bind(written, self.catalog)
        });
        match binding {
            NamespaceBinding::Owned(owner) => Some(owner),
            NamespaceBinding::Foreign => None,
            NamespaceBinding::Rejected { mut diagnostic, .. } => {
                diagnostic.labels.insert(
                    0,
                    SourceLabel::new(
                        span.file_id,
                        span.range,
                        format!("cannot bind annotation `{written}`"),
                    ),
                );
                diagnostics.push(diagnostic);
                None
            }
            NamespaceBinding::Unknown => {
                let mut candidates = self
                    .catalog
                    .candidate_ids(short)
                    .filter(|id| !trait_only || self.catalog.alias_has_trait(short, id))
                    .collect::<Vec<_>>();
                if !trait_only && short == "Derive" {
                    candidates.push("dust_dart::Derive");
                    candidates.sort_unstable();
                    candidates.dedup();
                }
                match candidates.as_slice() {
                    [id] => Some(SymbolId::new(*id)),
                    [] => None,
                    _ => {
                        diagnostics.push(Diagnostic::error(format!("annotation `{written}` has unknown provenance and multiple registered owners: {}", candidates.join(", "))).with_label(SourceLabel::new(span.file_id, span.range, "use a visible declaration with known ownership")));
                        None
                    }
                }
            }
        }
    }
}
