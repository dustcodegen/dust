use dust_parser_dart::ParsedCombinatorKind;

use super::*;

/// Owner namespaces and part declarations share one library identity.
mod parts;

impl NamespaceGraph {
    /// Look up a name through a root or part's owning library namespace.
    pub(super) fn lookup(
        &self,
        uri: &str,
        prefix: Option<&str>,
        name: &str,
    ) -> Result<Worlds, NamespaceBinding> {
        let uri = self.owning_uri(uri)?;
        let unit = &self.units[uri];
        let mut local = self.local_world(uri, name);
        if !local.problems.is_empty() {
            return Ok(BTreeSet::from([local]));
        }
        if prefix.is_none() && !local.declarations.is_empty() {
            return Ok(BTreeSet::from([local]));
        }
        let imports = unit
            .imports
            .iter()
            .filter(|edge| edge.prefix.as_deref() == prefix)
            .collect::<Vec<_>>();
        if let Some(prefix) = prefix {
            if imports.is_empty() {
                if !self.local_world(uri, prefix).declarations.is_empty() {
                    return Err(NamespaceBinding::Unknown);
                }
                return Err(rejected(
                    NamespaceFailureKind::UndeclaredPrefix,
                    format!("annotation prefix `{prefix}` is not declared by an import"),
                    Vec::new(),
                ));
            }
            if !self.local_world(uri, prefix).declarations.is_empty() {
                return Err(rejected(
                    NamespaceFailureKind::Ambiguous,
                    format!("local declaration conflicts with import prefix `{prefix}`"),
                    Vec::new(),
                ));
            }
        } else if unit
            .imports
            .iter()
            .any(|edge| edge.prefix.as_deref() == Some(name))
        {
            return Err(rejected(
                NamespaceFailureKind::Hidden,
                format!("`{name}` is an import prefix, not an annotation declaration"),
                Vec::new(),
            ));
        }
        let exports = self.export_worlds(name);
        if prefix.is_some() {
            local.declarations.clear();
        }
        let mut worlds = BTreeSet::from([local]);
        for edge in imports {
            if !visible(edge, name) || name.starts_with('_') {
                continue;
            }
            let mut alternatives = self.edge_worlds(edge, &exports);
            if edge.deferred {
                for world in &mut alternatives {
                    world.problems.insert(Problem::new(
                        edge.label.file_id,
                        edge.label.range,
                        "metadata cannot use a deferred import".to_owned(),
                    ));
                }
            }
            worlds = combine(&worlds, &alternatives.into_iter().collect());
        }
        Ok(worlds)
    }

    /// Each relevant name gets a finite, cycle-aware fixed-point computation.
    fn export_worlds(&self, name: &str) -> Arc<ExportWorlds> {
        if let Some(cached) = self
            .exports
            .lock()
            .expect("namespace cache is not poisoned")
            .get(name)
        {
            return Arc::clone(cached);
        }
        let mut current: ExportWorlds = self
            .units
            .iter()
            .filter(|(_, unit)| unit.part_of.is_none())
            .map(|(uri, _)| {
                (
                    uri.clone(),
                    BTreeSet::from([if name.starts_with('_') {
                        World::default()
                    } else {
                        self.local_world(uri, name)
                    }]),
                )
            })
            .collect();
        let mut seen = BTreeSet::new();
        let mut failed = ExportWorlds::new();
        loop {
            let next: ExportWorlds = self
                .units
                .iter()
                .filter(|(_, unit)| unit.part_of.is_none())
                .map(|(uri, unit)| {
                    if let Some(worlds) = failed.get(uri) {
                        return (uri.clone(), worlds.clone());
                    }
                    let local = if name.starts_with('_') {
                        World::default()
                    } else {
                        self.local_world(uri, name)
                    };
                    let local_shadows = !local.declarations.is_empty();
                    let mut worlds = BTreeSet::from([local]);
                    if !name.starts_with('_') && !local_shadows {
                        for edge in &unit.exports {
                            if visible(edge, name) {
                                worlds = combine(
                                    &worlds,
                                    &self.edge_worlds(edge, &current).into_iter().collect(),
                                );
                            }
                        }
                    }
                    (uri.clone(), worlds)
                })
                .collect();
            if next == current {
                break;
            }
            if seen.contains(&current) {
                for (uri, worlds) in &next {
                    if current.get(uri) != Some(worlds) {
                        let problem = Problem::new(
                            self.units[uri].file_id,
                            TextRange::empty(0_u32),
                            format!(
                                "conditional namespace alternatives did not stabilize in `{uri}`"
                            ),
                        );
                        let mut world = self.local_world(uri, name);
                        world.problems.insert(problem);
                        failed.insert(uri.clone(), BTreeSet::from([world]));
                    }
                }
                seen.clear();
            } else {
                seen.insert(current);
            }
            current = next;
        }
        let result = Arc::new(current);
        let mut memo = self
            .exports
            .lock()
            .expect("namespace cache is not poisoned");
        Arc::clone(memo.entry(name.to_owned()).or_insert(result))
    }

    /// Preserve alternatives; never union mutually exclusive declarations together.
    fn edge_worlds(&self, edge: &Edge, exports: &ExportWorlds) -> Vec<World> {
        edge.targets
            .iter()
            .flat_map(|target| match target {
                Ok(uri) => exports
                    .get(uri)
                    .map(|worlds| worlds.iter().cloned().collect())
                    .unwrap_or_else(|| {
                        let message = if self
                            .units
                            .get(uri)
                            .is_some_and(|unit| unit.part_of.is_some())
                        {
                            format!(
                                "namespace import/export targets part `{uri}` instead of a library"
                            )
                        } else {
                            format!("missing required namespace input `{uri}`")
                        };
                        vec![World {
                            declarations: BTreeSet::new(),
                            problems: BTreeSet::from([Problem::new(
                                edge.label.file_id,
                                edge.label.range,
                                message,
                            )]),
                        }]
                    }),
                Err(problem) => vec![World {
                    declarations: BTreeSet::new(),
                    problems: BTreeSet::from([problem.clone()]),
                }],
            })
            .collect()
    }
}

/// Applying each show/hide in order is equivalent to filtering this one name.
fn visible(edge: &Edge, name: &str) -> bool {
    edge.combinators.iter().all(|clause| match clause.kind {
        ParsedCombinatorKind::Show => clause.names.iter().any(|shown| shown == name),
        ParsedCombinatorKind::Hide => !clause.names.iter().any(|hidden| hidden == name),
    })
}

/// Independent imports/exports coexist; conditional alternatives remain separate.
fn combine(left: &Worlds, right: &Worlds) -> Worlds {
    left.iter()
        .flat_map(|a| {
            right.iter().map(move |b| {
                let mut world = a.clone();
                world.declarations.extend(b.declarations.iter().cloned());
                world.problems.extend(b.problems.iter().cloned());
                if world
                    .declarations
                    .iter()
                    .any(|decl| !decl.uri.starts_with("dart:"))
                {
                    world
                        .declarations
                        .retain(|decl| !decl.uri.starts_with("dart:"));
                }
                world
            })
        })
        .collect()
}
