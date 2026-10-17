use super::*;

/// Part requests must promote their actual owner, never imports on a part.
#[cfg(test)]
mod tests;

impl Loader<'_> {
    /// Promote only namespaces actually queried while resolving constant provenance.
    pub(in crate::namespace_sources) fn require_lexical_imports(
        &mut self,
        uris: &BTreeSet<String>,
    ) -> Result<bool, String> {
        let mut promote = BTreeSet::new();
        for uri in uris {
            if let Some(owners) = self.part_owners.get(uri) {
                promote.extend(owners.clone());
            } else {
                promote.insert(uri.clone());
            }
        }
        let mut changed = false;
        for uri in promote {
            if self.imports.insert(uri.clone()) {
                self.expand(&uri)?;
                changed = true;
            }
        }
        if changed {
            self.drain()?;
        }
        Ok(changed)
    }

    /// Reuse one parse/read queue and retain the existing named-alias promotion rules.
    pub(super) fn drain(&mut self) -> Result<(), String> {
        loop {
            while let Some((uri, source)) = self.queue.pop_first() {
                let id = self.file_id(&uri)?;
                let unit = self.read(source, id);
                self.units.insert(uri.clone(), unit);
                self.expand(&uri)?;
            }
            let mut promote = BTreeSet::new();
            for (uri, unit) in &self.units {
                let Ok(unit) = unit else {
                    continue;
                };
                if !unit.parsed.typedefs.iter().any(|alias| {
                    alias
                        .parsed_aliased_type
                        .as_ref()
                        .is_some_and(|ty| matches!(ty.kind, ParsedTypeKind::Named { .. }))
                }) {
                    continue;
                }
                if let Some(owners) = self.part_owners.get(uri) {
                    promote.extend(owners.clone());
                } else {
                    promote.insert(uri.clone());
                }
            }
            let mut promoted = false;
            for uri in promote {
                if self.imports.insert(uri.clone()) {
                    self.expand(&uri)?;
                    promoted = true;
                }
            }
            if !promoted {
                break;
            }
        }
        Ok(())
    }
}
