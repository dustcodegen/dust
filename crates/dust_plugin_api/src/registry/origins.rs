use std::collections::BTreeMap;

use super::*;
use crate::AnnotationOrigin;

impl PluginRegistry {
    /// Returns deterministic, deduplicated origins for effectively owned claims.
    pub fn annotation_origins(&self) -> Vec<AnnotationOrigin> {
        let mut origins = self
            .plugins
            .iter()
            .flat_map(|entry| entry.annotation_origins.iter().copied())
            .collect::<Vec<_>>();
        origins.sort_unstable();
        origins.dedup();
        origins
    }

    /// Validate before registration mutates claim maps, including duplicate metadata.
    pub(super) fn validate_annotation_origins(
        &self,
        plugin: &dyn DustPlugin,
        fallback: bool,
    ) -> Result<Vec<AnnotationOrigin>, Diagnostic> {
        let mut known: BTreeMap<_, _> = self
            .annotation_origins()
            .into_iter()
            .map(|origin| (origin.symbol, origin))
            .collect();
        let mut validated = Vec::new();
        for origin in plugin.annotation_origins() {
            let owns_trait = plugin.claimed_traits().contains(&origin.symbol);
            let owns_config = plugin.claimed_configs().contains(&origin.symbol);
            if !owns_trait && !owns_config {
                return Err(Diagnostic::error(format!(
                    "plugin `{}` declares an origin for unclaimed symbol `{}`",
                    plugin.plugin_name(),
                    origin.symbol
                )));
            }
            if fallback
                && !(owns_trait
                    && !self
                        .trait_owners
                        .contains_key(&SymbolId::new(origin.symbol))
                    || owns_config
                        && !self
                            .config_owners
                            .contains_key(&SymbolId::new(origin.symbol)))
            {
                continue;
            }
            if let Some(previous) = known.insert(origin.symbol, *origin) {
                if previous != *origin {
                    return Err(Diagnostic::error(format!(
                        "conflicting declaration origins for symbol `{}`",
                        origin.symbol
                    )));
                }
            }
            validated.push(*origin);
        }
        validated.sort_unstable();
        validated.dedup();
        Ok(validated)
    }
}
