use dust_ir::SpanIr;
use dust_parser_dart::{
    ParsedLexicalDeclaration, ParsedLexicalDeclarationKind as Kind,
    ParsedLexicalInitializerKind as Value, ParsedLexicalScopeKind,
};

use super::*;

/// Fully retained library constants, separate from missing-fact legacy adapters.
mod library_values;
/// Qualified constants use actual type declarations, not import-prefix spelling.
mod qualified;

/// Source declaration identity bounds recursion through direct constant aliases.
type Active = BTreeSet<(u32, u32)>;

impl NamespaceGraph {
    /// Search the source's lexical chain before the existing library operation.
    pub(super) fn bind_at(
        &self,
        uri: &str,
        name: &str,
        catalog: &SymbolCatalog,
        span: SpanIr,
        active: &mut Active,
    ) -> NamespaceBinding {
        let Some(unit) = self
            .units
            .values()
            .find(|unit| unit.file_id == span.file_id)
        else {
            return rejected(
                NamespaceFailureKind::MissingInput,
                "metadata source is missing from the namespace graph",
                Vec::new(),
            );
        };
        let first = name.split('.').next().unwrap_or(name);
        let mut scope = unit.lexical_scopes.iter().rposition(|scope| {
            scope.visibility.start() <= span.range.start()
                && scope.visibility.end() >= span.range.end()
        });
        let mut seen = BTreeSet::new();
        while let Some(index) = scope {
            let Some(current) = unit
                .lexical_scopes
                .get(index)
                .filter(|_| seen.insert(index))
            else {
                return rejected(
                    NamespaceFailureKind::MissingInput,
                    "invalid lexical scope parent chain",
                    Vec::new(),
                );
            };
            if (current.kind != ParsedLexicalScopeKind::Library || !active.is_empty())
                && let Some(declaration) = current
                    .declarations
                    .iter()
                    .find(|declaration| declaration.name.name == first)
            {
                if name != first {
                    return unsupported(
                        unit,
                        declaration,
                        "qualified access through a lexical value is unsupported",
                    );
                }
                return self.constant(uri, unit, declaration, catalog, active);
            }
            scope = current.parent;
        }
        self.qualified_constant(uri, name, catalog, active)
            .or_else(|| self.library_constant(uri, name, catalog, active))
            .unwrap_or_else(|| self.bind_name(uri, name, catalog, &mut BTreeSet::new()))
    }

    /// Resolve only retained direct initializer facts, never arbitrary constant source.
    fn constant(
        &self,
        uri: &str,
        unit: &Unit,
        declaration: &ParsedLexicalDeclaration,
        catalog: &SymbolCatalog,
        active: &mut Active,
    ) -> NamespaceBinding {
        let key = (unit.file_id.raw(), declaration.name.span.start().to_u32());
        if !active.insert(key) {
            return unsupported(unit, declaration, "cyclic lexical annotation constant");
        }
        let result = match &declaration.kind {
            Kind::EnumValue => NamespaceBinding::Foreign,
            Kind::Constant(Some(initializer)) => {
                if !matches!(initializer.kind, Value::Opaque) {
                    self.lexical_imports
                        .lock()
                        .expect("lexical import requests are not poisoned")
                        .insert(uri.to_owned());
                }
                let span = SpanIr::new(unit.file_id, initializer.span);
                match &initializer.kind {
                    Value::Reference(names) => {
                        let name = names
                            .iter()
                            .map(|name| name.name.as_str())
                            .collect::<Vec<_>>()
                            .join(".");
                        self.bind_at(uri, &name, catalog, span, active)
                    }
                    Value::Constructor { callee, .. } => {
                        let name = callee
                            .iter()
                            .map(|name| name.name.as_str())
                            .collect::<Vec<_>>()
                            .join(".");
                        self.constructor_value(uri, &name, catalog, span, active)
                    }
                    Value::Opaque => unsupported(
                        unit,
                        declaration,
                        "opaque lexical annotation constant provenance is unsupported",
                    ),
                }
            }
            Kind::Constant(None) | Kind::TypeParameter | Kind::Member => unsupported(
                unit,
                declaration,
                "lexical annotation value has no supported constant provenance",
            ),
        };
        active.remove(&key);
        match result {
            NamespaceBinding::Owned(_) => unsupported(
                unit,
                declaration,
                "Dust annotation constants are unsupported; use a direct annotation with its arguments",
            ),
            NamespaceBinding::Unknown => unsupported(
                unit,
                declaration,
                "lexical annotation constant has unknown provenance; imported-name fallback is forbidden",
            ),
            result => result,
        }
    }
}

/// Fail with the actual local declaration rather than rebinding an imported symbol.
fn unsupported(
    unit: &Unit,
    declaration: &ParsedLexicalDeclaration,
    message: &str,
) -> NamespaceBinding {
    rejected(
        NamespaceFailureKind::MissingInput,
        message,
        vec![SourceLabel::new(
            unit.file_id,
            declaration.span,
            format!(
                "lexical declaration `{}` shadows imported names",
                declaration.name.name
            ),
        )],
    )
}
