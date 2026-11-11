use super::*;

impl NamespaceGraph {
    /// Nested local values and aliases request imports only when their target leaves the library.
    pub(in crate::namespace) fn request_imports_for(&self, uri: &str, name: &str) {
        let first = name.split('.').next().unwrap_or(name);
        let local = self.owning_uri(uri).ok().is_some_and(|owner| {
            let world = self.local_world(owner, first);
            world.problems.is_empty()
                && world.declarations.len() == 1
                && !(name.contains('.')
                    && self.units[owner]
                        .imports
                        .iter()
                        .any(|edge| edge.prefix.as_deref() == Some(first)))
        });
        if !local {
            self.lexical_imports
                .lock()
                .expect("lexical import requests are not poisoned")
                .insert(uri.to_owned());
        }
    }
}
