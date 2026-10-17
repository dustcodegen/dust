use std::path::Path;

use dust_parser_dart::ParsedDirective;
use dust_resolver::{BindingContext, PrimaryPart, SymbolCatalog};

use super::NamespaceSources;

impl NamespaceSources {
    /// Binds annotations and the primary part using the same immutable source graph.
    pub(crate) fn bindings<'a>(
        &'a self,
        source: &Path,
        output: &Path,
        catalog: &'a SymbolCatalog,
    ) -> BindingContext<'a> {
        let unit = self
            .unit(source)
            .expect("discovered namespace root has source facts");
        let primary = unit
            .parsed
            .directives
            .iter()
            .find_map(|directive| {
                let ParsedDirective::Part {
                    uri,
                    uri_literal: Some(literal),
                    span,
                } = directive
                else {
                    return None;
                };
                let reference = literal.value()?;
                let target = unit.targets.get(&reference)?.as_ref().ok()?;
                (target.path == output).then_some(PrimaryPart::Resolved { uri, span: *span })
            })
            .unwrap_or(PrimaryPart::Missing);
        BindingContext::new(catalog, self.scope(source)).with_primary_part(primary)
    }
}
