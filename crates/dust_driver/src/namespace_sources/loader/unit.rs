use dust_parser_dart::{ParseResult, ParsedUriLiteral};
use dust_text::TextRange;

use super::*;

impl Loader<'_> {
    /// Installs parsed facts and exact directive identities without mutating the queue.
    pub(super) fn resolve(
        &self,
        source: Source,
        id: FileId,
        input: SourceInput,
        parsed: ParseResult,
        targets: Option<ResolvedTargets>,
    ) -> Unit {
        let SourceInput {
            text,
            owner,
            generated_library,
        } = input;
        let mut surface = parsed.library;
        let targets = if let Some(owner) = &owner {
            surface.directives.push(ParsedDirective::PartOf {
                uri_literal: Some(ParsedUriLiteral {
                    source: dust_dart_emit::dart_string_literal(owner),
                    span: TextRange::empty(0_u32),
                }),
                library_name: None,
                uri: Some(owner.clone()),
                span: TextRange::empty(0_u32),
            });
            // Owned placeholders contain only this synthetic part-of directive.
            let target = self
                .units
                .get(owner)
                .and_then(|unit| unit.as_ref().ok())
                .map(|unit| Ok(unit.source.clone()))
                .unwrap_or_else(|| self.index.resolve(&source, owner));
            BTreeMap::from([(owner.clone(), target)])
        } else {
            targets.unwrap_or_else(|| resolve_targets(self.index, &source, &surface))
        };
        Unit {
            source,
            file_id: id,
            text,
            content_digest: OnceLock::new(),
            parsed: surface,
            diagnostics: parsed.diagnostics,
            namespace_diagnostics: parsed.namespace_diagnostics,
            targets,
            dependencies: BTreeSet::new(),
            generated_owner: owner,
            generated_library,
        }
    }
}

/// Generated ownership retains exact existing sources and a guarded fallback.
#[cfg(test)]
mod tests;
