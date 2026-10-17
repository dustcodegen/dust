use super::*;

impl NamespaceGraph {
    /// Follow the defining unit's facts even for imported or re-exported constants.
    pub(super) fn library_constant(
        &self,
        uri: &str,
        name: &str,
        catalog: &SymbolCatalog,
        active: &mut Active,
    ) -> Option<NamespaceBinding> {
        let (prefix, short) = name
            .rsplit_once('.')
            .map_or((None, name), |(prefix, short)| (Some(prefix), short));
        let worlds = self.lookup(uri, prefix, short).ok()?;
        let mut inspected = false;
        let bindings = worlds
            .iter()
            .map(|world| {
                if world.problems.is_empty() && world.declarations.len() == 1 {
                    let declaration = world
                        .declarations
                        .first()
                        .expect("checked declaration count");
                    if let Some(unit) = self
                        .units
                        .values()
                        .find(|unit| unit.file_id == declaration.file_id)
                    {
                        if let Some(value) = unit
                            .lexical_scopes
                            .iter()
                            .filter(|scope| scope.kind == ParsedLexicalScopeKind::Library)
                            .flat_map(|scope| &scope.declarations)
                            .find(|value| value.name.name == declaration.name)
                        {
                            inspected = true;
                            return self.constant(&declaration.uri, unit, value, catalog, active);
                        }
                    }
                }
                self.bind_world(world, short, catalog, &mut BTreeSet::new())
            })
            .collect::<Vec<_>>();
        if !inspected {
            return None;
        }
        agree(name, &bindings, || {
            worlds
                .iter()
                .flat_map(|world| world.declarations.iter().map(Declaration::label))
                .collect()
        })
    }
}
