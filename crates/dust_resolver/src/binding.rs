use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::{AnnotationIr, AnnotationValueIr, NameIr, SpanIr, SymbolId, TraitApplicationIr};
use dust_parser_dart::{ParsedAnnotation, ParsedMethodParamSurface};
use dust_text::FileId;

use crate::{NamespaceBinding, NamespaceScope, PrimaryPart, ResolvedSymbol, SymbolCatalog};

/// Shared ownership decision before selecting a semantic role or normalizing.
pub struct BindingContext<'a> {
    /// Effective canonical symbols and declaration origins.
    pub(crate) catalog: &'a SymbolCatalog,
    /// Absent only for the explicit context-free compatibility adapter.
    pub(crate) namespace: Option<NamespaceScope<'a>>,
    /// `None` preserves context-free part matching; `Some` is a resolved decision.
    pub(crate) primary_part: Option<PrimaryPart<'a>>,
}

/// The intrinsic grouping marker is not a plugin trait/config claim.
pub(crate) enum Application<'a> {
    /// Scoped groups already carry validated traits; legacy declarations fill them.
    Group(Vec<TraitApplicationIr>),
    /// One bound owner with its selected semantic role.
    Symbol(&'a ResolvedSymbol),
    /// Foreign, unsupported or diagnosed metadata has no application.
    Unowned,
}

impl<'a> BindingContext<'a> {
    /// Shares one explicit binding mode across normalization and IR lowering.
    ///
    /// `None` selects the context-free compatibility adapter, not an error fallback.
    pub fn new(catalog: &'a SymbolCatalog, namespace: Option<NamespaceScope<'a>>) -> Self {
        Self {
            catalog,
            namespace,
            primary_part: None,
        }
    }

    /// Carries the source loader's primary-target proof into shared resolution.
    pub fn with_primary_part(mut self, primary_part: PrimaryPart<'a>) -> Self {
        self.primary_part = Some(primary_part);
        self
    }

    /// Whether source namespace ownership is enforced rather than inferred.
    pub fn has_namespace(&self) -> bool {
        self.namespace.is_some()
    }

    /// Retains raw annotation metadata while applying this context's owner policy.
    pub fn annotation_ir(
        &self,
        file_id: FileId,
        annotation: &ParsedAnnotation,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> AnnotationIr {
        let owner = if self.namespace.is_some() {
            self.annotation_owner(file_id, annotation, diagnostics)
        } else {
            self.catalog
                .resolve_qualified(&annotation.qualified_name)
                .or_else(|| self.catalog.resolve(&annotation.name))
                .map(|resolved| resolved.symbol.clone())
        };
        let ir = crate::annotation_ir_from_parsed(file_id, annotation, owner);
        if self.namespace.is_some()
            && ir
                .resolved_symbol
                .as_ref()
                .is_some_and(|id| id.0 == "dust_dart::Derive")
        {
            let _ = self.group_traits(file_id, annotation, &ir.positional_args, diagnostics);
        }
        ir
    }

    /// Uses the same trait/config parameter resolution as class methods.
    pub fn parameter(
        &self,
        file_id: FileId,
        parameter: &ParsedMethodParamSurface,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> crate::ResolvedMethodParam {
        crate::resolve_support::resolve_method_param(file_id, parameter, self, diagnostics)
    }

    /// Raw parsed names and spans are never rewritten to canonical strings.
    pub(crate) fn application(
        &self,
        file_id: FileId,
        annotation: &ParsedAnnotation,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Application<'a> {
        if self.namespace.is_none() {
            if annotation.is_named("Derive") {
                return Application::Group(Vec::new());
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
        let owner = self.annotation_owner(file_id, annotation, diagnostics);
        match owner.as_ref().map(|id| id.0.as_str()) {
            Some("dust_dart::Derive") => {
                let (values, _) =
                    crate::annotations::annotation_argument_values(file_id, annotation);
                Application::Group(self.group_traits(file_id, annotation, &values, diagnostics))
            }
            Some(id) => self
                .catalog
                .resolve_qualified(id)
                .map_or(Application::Unowned, Application::Symbol),
            None => Application::Unowned,
        }
    }

    /// Validate nested ownership once, including sites that do not apply derives.
    pub(crate) fn group_traits(
        &self,
        file_id: FileId,
        annotation: &ParsedAnnotation,
        values: &[AnnotationValueIr],
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Vec<TraitApplicationIr> {
        let mut traits = Vec::new();
        for name in crate::resolve_support::derive_member_names(values) {
            let diagnostic_count = diagnostics.len();
            match self.derive_trait(name, diagnostics) {
                Some(resolved) => traits.push(TraitApplicationIr {
                    symbol: resolved.symbol.clone(),
                    span: SpanIr::new(file_id, annotation.span),
                }),
                None if diagnostics.len() == diagnostic_count => diagnostics.push(
                    Diagnostic::warning(format!(
                        "unknown derive trait or config `{}`",
                        if self.namespace.is_some() {
                            &name.source
                        } else {
                            &name.short
                        }
                    ))
                    .with_label(SourceLabel::new(
                        file_id,
                        if self.namespace.is_some() {
                            name.span.range
                        } else {
                            annotation.span
                        },
                        "annotation member is not owned by any registered symbol",
                    )),
                ),
                None => {}
            }
        }
        traits
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
        let lookup = name.prefix.as_ref().map_or_else(
            || name.short.clone(),
            |prefix| format!("{prefix}.{}", name.short),
        );
        let owner = self.owner(
            &name.source,
            &lookup,
            &name.short,
            name.span,
            true,
            diagnostics,
        )?;
        self.catalog.resolve_qualified_trait(&owner.0)
    }

    /// Bind semantic identifiers without rewriting source spelling or spans.
    fn annotation_owner(
        &self,
        file_id: FileId,
        annotation: &ParsedAnnotation,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SymbolId> {
        let lookup = annotation.prefix.as_ref().map_or_else(
            || annotation.name.clone(),
            |prefix| format!("{prefix}.{}", annotation.name),
        );
        self.owner(
            &annotation.qualified_name,
            &lookup,
            &annotation.name,
            SpanIr::new(file_id, annotation.span),
            false,
            diagnostics,
        )
    }

    /// Unknown provenance alone permits unique-owner legacy compatibility.
    pub(crate) fn owner(
        &self,
        written: &str,
        lookup: &str,
        short: &str,
        span: SpanIr,
        trait_only: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SymbolId> {
        let binding = self.namespace.map_or(NamespaceBinding::Unknown, |scope| {
            scope.bind(lookup, self.catalog)
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
