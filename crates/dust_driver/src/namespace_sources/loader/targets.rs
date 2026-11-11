use std::collections::BTreeMap;

use dust_parser_dart::ParsedDartFileSurface;

use super::{Source, SourceIndex, implicit_core, literals};

/// Immutable directive identities and resolution errors for one parsed source.
pub(super) type ResolvedTargets = BTreeMap<String, Result<Source, String>>;

/// Resolves structured directives, including implicit core, without reading files.
pub(super) fn resolve_targets(
    index: &SourceIndex,
    source: &Source,
    surface: &ParsedDartFileSurface,
) -> ResolvedTargets {
    let core = implicit_core(&surface.directives, &source.uri);
    let mut targets = BTreeMap::new();
    for directive in surface.directives.iter().chain(core.as_ref()) {
        for literal in literals(directive) {
            if let Some(reference) = literal.value() {
                let target = index.resolve(source, &reference);
                targets.insert(reference, target);
            }
        }
    }
    targets
}
