use super::*;

/// Owning normalized library facts, prepared independently of a namespace graph.
///
/// URI callback failures remain source-attached edge facts. Identity conflicts
/// are checked when this value is inserted into its destination graph.
#[derive(Debug)]
pub struct PreparedNamespaceLibrary {
    /// Canonical identity supplied by the caller.
    uri: String,
    /// Immutable namespace facts copied from the parser surface.
    unit: Unit,
}

impl NamespaceGraph {
    /// Normalizes one library without reading or changing graph or catalog state.
    ///
    /// Independent libraries may be prepared concurrently, then inserted in the
    /// caller's original order. Written directives precede the optional appended one.
    pub fn prepare_library(
        uri: impl Into<String>,
        file_id: FileId,
        parsed: &ParsedDartFileSurface,
        appended_directive: Option<&ParsedDirective>,
        mut resolve_uri: impl FnMut(&str, &str) -> Result<String, String>,
    ) -> PreparedNamespaceLibrary {
        let uri = uri.into();
        let unit = input::unit(&uri, file_id, parsed, appended_directive, &mut resolve_uri);
        PreparedNamespaceLibrary { uri, unit }
    }

    /// Inserts prepared facts once, retaining duplicate errors and cache invalidation.
    pub fn insert_prepared_library(
        &mut self,
        prepared: PreparedNamespaceLibrary,
    ) -> Result<(), Diagnostic> {
        let PreparedNamespaceLibrary { uri, unit } = prepared;
        self.check_input_identity(&uri, unit.file_id)?;
        self.file_uris.insert(unit.file_id, uri.clone());
        self.units.insert(uri, unit);
        self.exports
            .get_mut()
            .expect("namespace cache is not poisoned")
            .clear();
        self.lookups.clear();
        Ok(())
    }

    /// Existing insertion must reject duplicates before invoking a URI callback.
    pub(super) fn check_input_identity(
        &self,
        uri: &str,
        file_id: FileId,
    ) -> Result<(), Diagnostic> {
        if self.units.contains_key(uri) || self.file_uris.contains_key(&file_id) {
            return Err(Diagnostic::error(format!(
                "duplicate namespace input identity `{uri}` or file id {}",
                file_id.raw()
            )));
        }
        Ok(())
    }
}

/// Prepared and immediate insertion retain exact binding and diagnostic contracts.
#[cfg(test)]
mod tests;
