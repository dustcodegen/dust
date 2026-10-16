use std::collections::{BTreeMap, BTreeSet};

use dust_ir::SymbolId;

/// The semantic role owned by a registered Dust symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// A trait-like symbol applied to a class.
    Trait,
    /// A configuration-like symbol applied to a class.
    Config,
}

/// One registered symbol plus its semantic role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSymbol {
    /// The fully qualified semantic symbol identifier.
    pub symbol: SymbolId,
    /// The role this symbol plays in the pipeline.
    pub kind: SymbolKind,
}

/// A lookup table from surface annotation names to fully qualified Dust symbols.
///
/// ```
/// use dust_resolver::SymbolCatalog;
/// let mut catalog = SymbolCatalog::new();
/// catalog.register_config("GET", "dust_dart::GET");
/// catalog.register_config("GET", "dust_server::GET");
/// assert!(catalog.resolve_config("GET").is_none());
/// assert!(catalog.resolve_qualified_config("dust_dart::GET").is_some());
/// assert_eq!(catalog.candidate_ids("GET").collect::<Vec<_>>(),
///            ["dust_dart::GET", "dust_server::GET"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SymbolCatalog {
    /// All semantic roles, indexed by canonical identity rather than short name.
    by_symbol: BTreeMap<String, ResolvedSymbols>,
    /// Surface aliases retain the owners registered for each role.
    by_name: BTreeMap<String, SymbolCandidates>,
}

/// Trait/config roles retained for one canonical identity.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ResolvedSymbols {
    /// Trait role for this canonical identity.
    trait_symbol: Option<ResolvedSymbol>,
    /// Config role for this canonical identity.
    config_symbol: Option<ResolvedSymbol>,
}

/// Role-specific owners registered for one surface alias.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct SymbolCandidates {
    /// Canonical identities registered as traits under this alias.
    trait_ids: BTreeSet<String>,
    /// Canonical identities registered as configs under this alias.
    config_ids: BTreeSet<String>,
}

impl SymbolCatalog {
    /// Creates an empty symbol catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one trait symbol without replacing distinct canonical owners.
    ///
    /// Returns the previous registration of the same canonical role, if any.
    pub fn register_trait(
        &mut self,
        annotation_name: impl Into<String>,
        symbol: impl Into<String>,
    ) -> Option<ResolvedSymbol> {
        self.register(annotation_name.into(), symbol.into(), SymbolKind::Trait)
    }

    /// Registers one config symbol without replacing distinct canonical owners.
    ///
    /// Returns the previous registration of the same canonical role, if any.
    pub fn register_config(
        &mut self,
        annotation_name: impl Into<String>,
        symbol: impl Into<String>,
    ) -> Option<ResolvedSymbol> {
        self.register(annotation_name.into(), symbol.into(), SymbolKind::Config)
    }

    /// Looks up one surface name only when its canonical owner is unique.
    ///
    /// For a unique owner registered in both roles, retains trait-first lookup.
    pub fn resolve(&self, annotation_name: &str) -> Option<&ResolvedSymbol> {
        let candidates = self.by_name.get(annotation_name)?;
        let id = unique(candidates.trait_ids.union(&candidates.config_ids))?;
        if candidates.trait_ids.contains(id) {
            self.resolve_qualified_trait(id)
        } else {
            self.resolve_qualified_config(id)
        }
    }

    /// Looks up a retained canonical identity, preserving config-first lookup.
    pub fn resolve_qualified(&self, symbol_name: &str) -> Option<&ResolvedSymbol> {
        self.resolve_qualified_config(symbol_name)
            .or_else(|| self.resolve_qualified_trait(symbol_name))
    }

    /// Looks up a canonical config symbol name.
    pub fn resolve_qualified_config(&self, symbol_name: &str) -> Option<&ResolvedSymbol> {
        self.by_symbol.get(symbol_name)?.config_symbol.as_ref()
    }

    /// Looks up a canonical trait symbol name.
    pub fn resolve_qualified_trait(&self, symbol_name: &str) -> Option<&ResolvedSymbol> {
        self.by_symbol.get(symbol_name)?.trait_symbol.as_ref()
    }

    /// Looks up a surface trait only when exactly one eligible owner exists.
    pub fn resolve_trait(&self, annotation_name: &str) -> Option<&ResolvedSymbol> {
        let id = unique(self.by_name.get(annotation_name)?.trait_ids.iter())?;
        self.resolve_qualified_trait(id)
    }

    /// Looks up a surface config only when exactly one eligible owner exists.
    pub fn resolve_config(&self, annotation_name: &str) -> Option<&ResolvedSymbol> {
        let id = unique(self.by_name.get(annotation_name)?.config_ids.iter())?;
        self.resolve_qualified_config(id)
    }

    /// Enumerates unique canonical candidates in deterministic identity order.
    ///
    /// Both roles of the same owner count once. Callers must bind or diagnose
    /// multiple candidates; this iterator does not choose an annotation owner.
    pub fn candidate_ids(&self, annotation_name: &str) -> impl Iterator<Item = &str> {
        self.by_name
            .get(annotation_name)
            .into_iter()
            .flat_map(|candidates| candidates.trait_ids.union(&candidates.config_ids))
            .map(String::as_str)
    }

    /// Retains a role and adds its surface alias without discarding other owners.
    fn register(
        &mut self,
        name: String,
        symbol: String,
        kind: SymbolKind,
    ) -> Option<ResolvedSymbol> {
        let candidates = self.by_name.entry(name).or_default();
        let ids = match kind {
            SymbolKind::Trait => &mut candidates.trait_ids,
            SymbolKind::Config => &mut candidates.config_ids,
        };
        ids.insert(symbol.clone());
        let roles = self.by_symbol.entry(symbol.clone()).or_default();
        let slot = match kind {
            SymbolKind::Trait => &mut roles.trait_symbol,
            SymbolKind::Config => &mut roles.config_symbol,
        };
        slot.replace(ResolvedSymbol {
            symbol: SymbolId::new(symbol),
            kind,
        })
    }
}

/// Returns a value only when the candidate set contains exactly one entry.
fn unique<T>(mut values: impl Iterator<Item = T>) -> Option<T> {
    let first = values.next()?;
    values.next().is_none().then_some(first)
}
