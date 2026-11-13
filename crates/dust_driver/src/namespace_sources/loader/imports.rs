use super::*;

/// Part requests must promote their actual owner, never imports on a part.
#[cfg(test)]
mod tests;

impl Loader<'_> {
    /// Promote only namespaces queried through constants or transparent type aliases.
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

    /// Checks the same owner promotion without changing source or import state.
    pub(in crate::namespace_sources) fn imports_ready(&self, uris: &BTreeSet<String>) -> bool {
        uris.iter().all(|uri| {
            self.part_owners.get(uri).map_or_else(
                || self.imports.contains(uri),
                |owners| owners.iter().all(|owner| self.imports.contains(owner)),
            )
        })
    }

    /// Reuses one parse/read queue for imports already required by real lookups.
    pub(super) fn drain(&mut self) -> Result<(), String> {
        let policy = Arc::clone(&self.parse_policy);
        let imports = self.imports.clone();
        let known = KnownSources {
            loaded: self.units.keys().cloned().collect(),
            cached: self.prepared_roots.uris().cloned().collect(),
        };
        ParseAhead::run(
            self.workers,
            self.index,
            self.potential,
            &policy,
            imports,
            known,
            |ahead| self.drain_queue(ahead),
        )
    }

    /// Preserves queue ordering, including entries newly inserted by expansion.
    fn drain_queue(&mut self, mut ahead: Option<&mut ParseAhead>) -> Result<(), String> {
        while !self.queue.is_empty() {
            if let Some(ahead) = ahead.as_deref_mut() {
                ahead.fill(&self.prepare_queue, &self.generated);
            }
            let (uri, source) = self.pop_source().expect("nonempty source queue");
            let id = self.file_id(&uri)?;
            let unit = self.read(source, id, ahead.as_deref_mut());
            self.units.insert(uri.clone(), unit);
            self.revision += 1;
            self.expand(&uri)?;
        }
        Ok(())
    }
}
