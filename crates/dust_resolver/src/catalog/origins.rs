use dust_diagnostics::Diagnostic;

use super::*;

/// The grouping annotation is an intrinsic, not a plugin trait/config claim.
const DERIVE_ORIGIN: AnnotationOrigin = AnnotationOrigin::new(
    "dust_dart::Derive",
    "package:dust_dart/src/derive/base.dart",
    "Derive",
);

impl SymbolCatalog {
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
        if let Some(previous) = self.annotation_origin(origin.symbol) {
            if previous != &origin {
                return Err(Diagnostic::error(format!(
                    "conflicting declaration origins for symbol `{}`",
                    origin.symbol
                )));
            }
        }
        self.origins.insert(origin.symbol.to_owned(), origin);
        Ok(())
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
