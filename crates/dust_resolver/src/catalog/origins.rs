use dust_diagnostics::Diagnostic;

use super::*;

/// The grouping annotation is an intrinsic, not a plugin trait/config claim.
const DERIVE_ORIGIN: AnnotationOrigin = AnnotationOrigin::new(
    "dust_dart::Derive",
    "package:dust_dart/src/derive/base.dart",
    "Derive",
);

impl Default for SymbolCatalog {
    fn default() -> Self {
        let mut catalog = Self {
            by_symbol: BTreeMap::new(),
            by_name: BTreeMap::new(),
            origins: BTreeMap::new(),
            by_origin: BTreeMap::new(),
        };
        catalog.index_origin(DERIVE_ORIGIN);
        catalog
    }
}

impl SymbolCatalog {
    /// Enumerates explicit declarations, including the intrinsic grouping marker.
    pub fn annotation_origins(&self) -> impl Iterator<Item = &AnnotationOrigin> {
        std::iter::once(&DERIVE_ORIGIN).chain(
            self.origins
                .values()
                .filter(|origin| origin.symbol != DERIVE_ORIGIN.symbol),
        )
    }

    /// Adds an explicit declaration only for a registered semantic owner.
    ///
    /// Identical repeats are harmless; conflicting declarations never replace
    /// known provenance. This does not resolve imports or establish visibility.
    pub fn register_annotation_origin(
        &mut self,
        origin: AnnotationOrigin,
    ) -> Result<(), Diagnostic> {
        if self.resolve_qualified(origin.symbol).is_none() {
            return Err(Diagnostic::error(format!(
                "annotation origin references unregistered symbol `{}`",
                origin.symbol
            )));
        }
        if let Some(previous) = self.annotation_origin(origin.symbol)
            && previous != &origin
        {
            return Err(Diagnostic::error(format!(
                "conflicting declaration origins for symbol `{}`",
                origin.symbol
            )));
        }
        self.index_origin(origin);
        self.origins.insert(origin.symbol.to_owned(), origin);
        Ok(())
    }

    /// Enumerates one declaration's canonical owners in deterministic identity order.
    pub fn annotation_owners(
        &self,
        library_uri: &str,
        declaration: &str,
    ) -> impl Iterator<Item = &str> {
        self.by_origin
            .get(library_uri)
            .and_then(|declarations| declarations.get(declaration))
            .into_iter()
            .flat_map(|owners| owners.iter().copied())
    }

    /// Keeps lookup facts synchronized only after origin validation succeeds.
    fn index_origin(&mut self, origin: AnnotationOrigin) {
        self.by_origin
            .entry(origin.library_uri)
            .or_default()
            .entry(origin.declaration)
            .or_default()
            .insert(origin.symbol);
    }

    /// Returns a registered origin, or the intrinsic grouping declaration.
    ///
    /// `None` is unknown provenance, never evidence of a foreign declaration.
    pub fn annotation_origin(&self, symbol: &str) -> Option<&AnnotationOrigin> {
        if symbol == DERIVE_ORIGIN.symbol {
            Some(&DERIVE_ORIGIN)
        } else {
            self.origins.get(symbol)
        }
    }
}
