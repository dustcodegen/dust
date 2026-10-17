use super::*;

impl NamespaceGraph {
    /// Namespace conflicts are decided before canonical ownership or role choice.
    pub(super) fn bind_name(
        &self,
        uri: &str,
        name: &str,
        catalog: &SymbolCatalog,
        active: &mut BTreeSet<(String, String)>,
    ) -> NamespaceBinding {
        let key = (uri.to_owned(), name.to_owned());
        if !active.insert(key.clone()) {
            return NamespaceBinding::Unknown;
        }
        let result = self.bind_inner(uri, name, catalog, active);
        active.remove(&key);
        result
    }

    /// Resolve each conditional world, then require ownership equivalence.
    fn bind_inner(
        &self,
        uri: &str,
        name: &str,
        catalog: &SymbolCatalog,
        active: &mut BTreeSet<(String, String)>,
    ) -> NamespaceBinding {
        let (prefix, short) = name
            .rsplit_once('.')
            .map_or((None, name), |(p, n)| (Some(p), n));
        let worlds = match self.lookup(uri, prefix, short) {
            Ok(worlds) => worlds,
            Err(binding) => return binding,
        };
        let bindings = worlds
            .iter()
            .map(|world| self.bind_world(world, short, catalog, active))
            .collect::<Vec<_>>();
        if let Some(failure) = bindings.iter().find(|binding| matches!(binding, NamespaceBinding::Rejected { kind, .. } if *kind != NamespaceFailureKind::Hidden)) { return failure.clone(); }
        let first = bindings
            .first()
            .expect("a namespace has at least one alternative");
        if bindings.iter().all(|binding| binding == first) {
            return first.clone();
        }
        rejected(
            NamespaceFailureKind::Conditional,
            format!("annotation `{name}` has target-dependent conditional ownership"),
            worlds
                .iter()
                .flat_map(|world| world.declarations.iter().map(Declaration::label))
                .collect(),
        )
    }

    /// One possible world can contain a real ambiguity, not just a branch choice.
    pub(super) fn bind_world(
        &self,
        world: &World,
        name: &str,
        catalog: &SymbolCatalog,
        active: &mut BTreeSet<(String, String)>,
    ) -> NamespaceBinding {
        if !world.problems.is_empty() {
            let deferred = world
                .problems
                .iter()
                .any(|problem| problem.message == "metadata cannot use a deferred import");
            return rejected(
                if deferred {
                    NamespaceFailureKind::Deferred
                } else {
                    NamespaceFailureKind::MissingInput
                },
                world
                    .problems
                    .iter()
                    .map(|problem| problem.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
                world.problems.iter().map(Problem::label).collect(),
            );
        }
        if world.declarations.len() > 1 {
            return rejected(
                NamespaceFailureKind::Ambiguous,
                format!("annotation `{name}` is ambiguous between distinct declarations"),
                world.declarations.iter().map(Declaration::label).collect(),
            );
        }
        let Some(declaration) = world.declarations.first() else {
            return rejected(
                NamespaceFailureKind::Hidden,
                format!("annotation name `{name}` is not visible in this namespace"),
                Vec::new(),
            );
        };
        let owners = catalog
            .annotation_origins()
            .filter(|origin| {
                origin.library_uri == declaration.uri && origin.declaration == declaration.name
            })
            .map(|origin| origin.symbol)
            .collect::<BTreeSet<_>>();
        if owners.len() > 1 {
            return rejected(
                NamespaceFailureKind::Ambiguous,
                format!(
                    "declaration `{}` has multiple registered owners: {}",
                    declaration.name,
                    owners.into_iter().collect::<Vec<_>>().join(", ")
                ),
                vec![declaration.label()],
            );
        }
        if let Some(owner) = owners.first() {
            return NamespaceBinding::Owned(SymbolId::new(*owner));
        }
        let kind = self
            .units
            .values()
            .find(|unit| unit.file_id == declaration.file_id)
            .and_then(|unit| {
                unit.declarations
                    .iter()
                    .find(|(decl, _)| {
                        decl.name == declaration.name && decl.start == declaration.start
                    })
                    .map(|(_, kind)| kind)
            });
        match kind {
            Some(DeclarationKind::Alias { target, builtin }) => {
                match self.bind_name(&declaration.uri, target, catalog, active) {
                    NamespaceBinding::Rejected {
                        kind: NamespaceFailureKind::Hidden,
                        ..
                    } => {
                        if *builtin {
                            NamespaceBinding::Foreign
                        } else {
                            NamespaceBinding::Unknown
                        }
                    }
                    result => result,
                }
            }
            Some(DeclarationKind::Opaque) => NamespaceBinding::Unknown,
            Some(DeclarationKind::Foreign) => NamespaceBinding::Foreign,
            _ if catalog
                .candidate_ids(&declaration.name)
                .any(|id| catalog.annotation_origin(id).is_none()) =>
            {
                NamespaceBinding::Unknown
            }
            _ => NamespaceBinding::Foreign,
        }
    }
}
