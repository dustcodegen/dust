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
        let values = worlds
            .iter()
            .map(|world| {
                if !world.problems.is_empty() || world.declarations.len() != 1 {
                    return None;
                }
                let declaration = world.declarations.first()?;
                let unit = self.unit_by_file_id(declaration.file_id)?;
                let value = unit
                    .lexical_scopes
                    .iter()
                    .filter(|scope| scope.kind == ParsedLexicalScopeKind::Library)
                    .flat_map(|scope| &scope.declarations)
                    .find(|value| value.name.name == declaration.name)?;
                Some((&declaration.uri, unit, value))
            })
            .collect::<Vec<_>>();
        if values.iter().all(Option::is_none) {
            let mut active = BTreeSet::from([(uri.to_owned(), name.to_owned())]);
            return Some(self.bind_worlds(name, short, &worlds, catalog, &mut active));
        }
        let bindings = worlds
            .iter()
            .zip(values)
            .map(|(world, value)| {
                if let Some((uri, unit, value)) = value {
                    return self.constant(uri, unit, value, catalog, active);
                }
                self.bind_world(world, short, catalog, &mut BTreeSet::new())
            })
            .collect::<Vec<_>>();
        agree(name, &bindings, || {
            worlds
                .iter()
                .flat_map(|world| world.declarations.iter().map(Declaration::label))
                .collect()
        })
    }
}

#[cfg(test)]
mod tests;
