use std::path::PathBuf;

use dust_ir::SymbolId;

/// A potential standalone output, not proof of ownership or permission to emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedLibrary {
    /// Absolute output path calculated from a handwritten source path.
    pub path: PathBuf,
    /// Canonical config annotation required on a class in that source.
    pub producer: SymbolId,
}
