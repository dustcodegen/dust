use super::*;
use dust_parser_dart::ParsedLexicalScope;

/// The actual declaring type and source unit, including definitions in parts.
struct TypeScope<'a> {
    /// Canonical library identity used to resolve the member's initializer.
    uri: String,
    /// Unit retaining original declaration/source coordinates.
    unit: &'a Unit,
    /// Parser-owned member namespace of this exact declaration.
    scope: &'a ParsedLexicalScope,
}

impl NamespaceGraph {
    /// A known class qualifier cannot become an unknown static-member legacy alias.
    pub(super) fn qualified_constant(
        &self,
        uri: &str,
        name: &str,
        catalog: &SymbolCatalog,
        active: &mut Active,
    ) -> Option<NamespaceBinding> {
        let (type_name, member) = name.rsplit_once('.')?;
        let types = match self.lookup_types(uri, type_name, catalog, &mut BTreeSet::new()) {
            Ok(Some(types)) => types,
            Ok(None) => return None,
            Err(error) => return Some(error),
        };
        let mut results = Vec::new();
        for owner in &types {
            let result = if let Some(value) = owner
                .scope
                .declarations
                .iter()
                .find(|value| value.name.name == member)
            {
                self.constant(&owner.uri, owner.unit, value, catalog, active)
            } else {
                rejected(
                    NamespaceFailureKind::Hidden,
                    format!("annotation member `{member}` is not declared by `{type_name}`"),
                    vec![SourceLabel::new(
                        owner.unit.file_id,
                        owner.scope.span,
                        "qualifying type declared here",
                    )],
                )
            };
            results.push(result);
        }
        agree(name, &results, || {
            types
                .iter()
                .map(|owner| {
                    SourceLabel::new(
                        owner.unit.file_id,
                        owner.scope.span,
                        "qualifying type declared here",
                    )
                })
                .collect()
        })
    }

    /// A parsed named invocation may target a constructor of a proven declaring type.
    pub(super) fn constructor_value(
        &self,
        uri: &str,
        name: &str,
        catalog: &SymbolCatalog,
        span: SpanIr,
        active: &mut Active,
    ) -> NamespaceBinding {
        let direct = self.bind_at(uri, name, catalog, span, active);
        if !matches!(
            direct,
            NamespaceBinding::Unknown
                | NamespaceBinding::Rejected {
                    kind: NamespaceFailureKind::Hidden,
                    ..
                }
        ) {
            return direct;
        }
        let Some((type_name, _)) = name.rsplit_once('.') else {
            return direct;
        };
        match self.lookup_types(uri, type_name, catalog, &mut BTreeSet::new()) {
            Ok(Some(_)) => self.bind_name(uri, type_name, catalog, &mut BTreeSet::new()),
            Ok(None) => direct,
            Err(error) => error,
        }
    }

    /// Resolve class aliases structurally, retaining conditional source alternatives.
    fn lookup_types<'a>(
        &'a self,
        uri: &str,
        name: &str,
        catalog: &SymbolCatalog,
        active: &mut BTreeSet<(String, String)>,
    ) -> Result<Option<Vec<TypeScope<'a>>>, NamespaceBinding> {
        let key = (uri.to_owned(), name.to_owned());
        if !active.insert(key.clone()) {
            return Err(rejected(
                NamespaceFailureKind::MissingInput,
                "cyclic qualifying type alias in annotation",
                Vec::new(),
            ));
        }
        let result = self.lookup_types_inner(uri, name, catalog, active);
        active.remove(&key);
        result
    }

    /// Obtain the defining unit before looking for a type's member constants.
    fn lookup_types_inner<'a>(
        &'a self,
        uri: &str,
        name: &str,
        catalog: &SymbolCatalog,
        active: &mut BTreeSet<(String, String)>,
    ) -> Result<Option<Vec<TypeScope<'a>>>, NamespaceBinding> {
        let (prefix, short) = name
            .rsplit_once('.')
            .map_or((None, name), |(prefix, short)| (Some(prefix), short));
        let worlds = match self.lookup(uri, prefix, short) {
            Ok(worlds) => worlds,
            Err(NamespaceBinding::Rejected {
                kind: NamespaceFailureKind::Hidden | NamespaceFailureKind::UndeclaredPrefix,
                ..
            }) => return Ok(None),
            Err(error) => return Err(error),
        };
        let ownership = self.bind_name(uri, name, catalog, &mut BTreeSet::new());
        if matches!(ownership, NamespaceBinding::Rejected { kind, .. } if kind != NamespaceFailureKind::Hidden)
        {
            return Err(ownership);
        }
        let mut result = Vec::new();
        for world in &worlds {
            if !world.problems.is_empty() || world.declarations.len() > 1 {
                return Err(self.bind_name(uri, name, catalog, &mut BTreeSet::new()));
            }
            let Some(declaration) = world.declarations.first() else {
                return Ok(None);
            };
            let Some(unit) = self
                .units
                .values()
                .find(|unit| unit.file_id == declaration.file_id)
            else {
                return Err(rejected(
                    NamespaceFailureKind::MissingInput,
                    "qualifying annotation declaration source is missing",
                    vec![declaration.label()],
                ));
            };
            let kind = unit
                .declarations
                .iter()
                .find(|(item, _)| item.start == declaration.start && item.name == declaration.name)
                .map(|(_, kind)| kind);
            // Library-level opaque values retain the approved legacy adapter policy.
            if matches!(kind, Some(DeclarationKind::Opaque)) {
                return Ok(None);
            }
            if let Some(DeclarationKind::Alias { target, .. }) = kind {
                match self.lookup_types(&declaration.uri, target, catalog, active)? {
                    Some(types) => result.extend(types),
                    None => return Ok(None),
                }
            } else if let Some(scope) = unit.lexical_scopes.iter().find(|scope| {
                scope.kind == ParsedLexicalScopeKind::Type
                    && scope.span.start().to_u32() == declaration.start
                    && scope.span.end().to_u32() == declaration.end
            }) {
                result.push(TypeScope {
                    uri: declaration.uri.clone(),
                    unit,
                    scope,
                });
            } else {
                return Err(rejected(
                    NamespaceFailureKind::MissingInput,
                    "qualified annotation value has no supported type-member provenance",
                    vec![declaration.label()],
                ));
            }
        }
        Ok(Some(result))
    }
}
