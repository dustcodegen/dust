use super::*;

impl NamespaceGraph {
    /// A part inherits a real owning library, not an invented empty import list.
    pub(in super::super) fn owning_uri<'a>(
        &'a self,
        uri: &'a str,
    ) -> Result<&'a str, NamespaceBinding> {
        let Some(unit) = self.units.get(uri) else {
            return Err(rejected(
                NamespaceFailureKind::MissingInput,
                format!("missing required namespace input `{uri}`"),
                Vec::new(),
            ));
        };
        let Some(header) = &unit.part_of else {
            return Ok(uri);
        };
        if let PartOwner::Uri(Err(problem), _) = header {
            return Err(rejected(
                NamespaceFailureKind::MissingInput,
                &problem.message,
                vec![problem.label()],
            ));
        }
        let owners = self
            .units
            .iter()
            .filter(|(owner_uri, owner)| {
                owner.part_of.is_none()
                    && matches_owner(header, owner_uri, owner)
                    && owner
                        .parts
                        .iter()
                        .any(|(target, _)| target.as_ref().is_ok_and(|target| target == uri))
            })
            .map(|(uri, _)| uri.as_str())
            .collect::<Vec<_>>();
        if let [owner] = owners.as_slice() {
            return Ok(owner);
        }
        let label = match header {
            PartOwner::Name(_, label) | PartOwner::Uri(_, label) => label.clone(),
        };
        Err(rejected(
            NamespaceFailureKind::MissingInput,
            format!(
                "part `{uri}` requires exactly one matching owning library; found {}",
                owners.len()
            ),
            vec![label],
        ))
    }

    /// Owner and part declarations share the owner URI but retain defining spans.
    pub(in super::super) fn local_world(&self, uri: &str, name: &str) -> World {
        let unit = &self.units[uri];
        let mut world = World::default();
        add_declarations(&mut world, uri, unit, name);
        let mut seen = BTreeSet::new();
        for (target, label) in &unit.parts {
            let part_uri = match target {
                Ok(uri) => uri,
                Err(problem) => {
                    world.problems.insert(problem.clone());
                    continue;
                }
            };
            if let Some(PartOwner::Uri(Err(problem), _)) = self
                .units
                .get(part_uri)
                .and_then(|part| part.part_of.as_ref())
            {
                world.problems.insert(problem.clone());
                continue;
            }
            let problem = if !seen.insert(part_uri) {
                Some(format!("duplicate part URI `{part_uri}`"))
            } else {
                match self.units.get(part_uri) {
                    None => Some(format!("missing required namespace part `{part_uri}`")),
                    Some(part)
                        if part
                            .part_of
                            .as_ref()
                            .is_none_or(|header| !matches_owner(header, uri, unit)) =>
                    {
                        Some(format!(
                            "part `{part_uri}` does not name owning library `{uri}`"
                        ))
                    }
                    Some(part)
                        if !part.imports.is_empty()
                            || !part.exports.is_empty()
                            || !part.parts.is_empty() =>
                    {
                        Some(format!(
                            "part `{part_uri}` contains library-only directives"
                        ))
                    }
                    Some(part) => {
                        add_declarations(&mut world, uri, part, name);
                        None
                    }
                }
            };
            if let Some(message) = problem {
                world
                    .problems
                    .insert(Problem::new(label.file_id, label.range, message));
            }
        }
        world
    }
}

/// URI-form and named-form headers cannot silently point at another library.
fn matches_owner(header: &PartOwner, uri: &str, unit: &Unit) -> bool {
    match header {
        PartOwner::Uri(target, _) => target.as_ref().is_ok_and(|target| target == uri),
        PartOwner::Name(name, _) => unit.library_name.as_ref() == Some(name),
    }
}

/// Rebase only identity, never original file/byte coordinates.
fn add_declarations(world: &mut World, owner: &str, unit: &Unit, name: &str) {
    for declaration in unit.declarations.keys().filter(|decl| decl.name == name) {
        let mut declaration = declaration.clone();
        declaration.uri = owner.to_owned();
        world.declarations.insert(declaration);
    }
}
