use dust_ir::SpanIr;
use dust_parser_dart::ParsedLexicalDeclarationKind;

use super::*;

/// The unique concrete declaration behind a written type name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDeclarationOrigin {
    /// Canonical library URI, including the owner of a declaration in a part.
    pub library_uri: String,
    /// Declared type name without an import prefix.
    pub name: String,
}

impl crate::BindingContext<'_> {
    /// Proves a concrete type declaration using the loaded namespace graph.
    pub fn type_declaration_origin(
        &self,
        name: &str,
        span: SpanIr,
    ) -> Option<TypeDeclarationOrigin> {
        self.namespace?.type_declaration_origin(name, span)
    }
}

impl NamespaceScope<'_> {
    /// Resolves a concrete type declaration without following typedef aliases.
    ///
    /// Missing, deferred, ambiguous or disagreeing conditional inputs do not
    /// prove an origin. Generic binders retain their lexical shadowing.
    pub fn type_declaration_origin(
        &self,
        name: &str,
        span: SpanIr,
    ) -> Option<TypeDeclarationOrigin> {
        let source = self.graph.unit_by_file_id(span.file_id)?;
        if source.lexical_scopes.is_empty() {
            return None;
        }
        let first = name.split('.').next()?;
        if source.lexical_scopes.iter().any(|scope| {
            scope.visibility.start() <= span.range.start()
                && scope.visibility.end() >= span.range.end()
                && scope.declarations.iter().any(|declaration| {
                    declaration.name.name == first
                        && matches!(
                            declaration.kind,
                            ParsedLexicalDeclarationKind::TypeParameter
                        )
                })
        }) {
            return None;
        }
        let (prefix, short) = name
            .rsplit_once('.')
            .map_or((None, name), |(prefix, short)| (Some(prefix), short));
        let worlds = self.graph.lookup(self.uri, prefix, short).ok()?;
        let declaration = worlds.first()?.declarations.first()?;
        if worlds.iter().any(|world| {
            !world.problems.is_empty()
                || world.declarations.len() != 1
                || world.declarations.first() != Some(declaration)
        }) {
            return None;
        }
        let defining = self.graph.unit_by_file_id(declaration.file_id)?;
        let kind = defining
            .declarations
            .iter()
            .find(|(candidate, _)| {
                candidate.name == declaration.name && candidate.start == declaration.start
            })
            .map(|(_, kind)| kind)?;
        matches!(kind, DeclarationKind::Concrete).then(|| TypeDeclarationOrigin {
            library_uri: declaration.uri.clone(),
            name: declaration.name.clone(),
        })
    }
}

#[cfg(test)]
mod tests;
