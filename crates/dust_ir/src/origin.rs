/// Explicit Dart declaration identity for one registered annotation symbol.
///
/// Public entrypoints/re-exports are not declaration origins. Missing metadata
/// means unknown provenance, never proof that a declaration is foreign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnnotationOrigin {
    /// Canonical semantic symbol, independent of its source package spelling.
    pub symbol: &'static str,
    /// Package URI of the library that actually declares the annotation.
    pub library_uri: &'static str,
    /// Class or typedef name declared in that library.
    pub declaration: &'static str,
}

impl AnnotationOrigin {
    /// Declares an origin without inferring any component from another.
    pub const fn new(
        symbol: &'static str,
        library_uri: &'static str,
        declaration: &'static str,
    ) -> Self {
        Self {
            symbol,
            library_uri,
            declaration,
        }
    }
}
