use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::Arc,
};

use dust_diagnostics::Diagnostic;
use dust_parser_dart::{
    ParsedDartFileSurface, ParsedDirective, ParsedTypeKind, ParsedUriLiteral,
    parse_file_with_backend,
};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_text::{FileId, SourceText, TextRange};

use super::index::{Source, SourceIndex};

/// Graph-only implicit imports and exact directive URI extraction.
mod directives;
use directives::literals;
pub(super) use directives::namespace_surface;

/// One read source snapshot, shared by graph construction and generation.
pub(super) struct Unit {
    /// Effective identity, physical path and language version.
    pub(super) source: Source,
    /// Stable diagnostic ID within this source graph snapshot.
    pub(super) file_id: FileId,
    /// Exact retained source text, empty for excluded Dust outputs.
    pub(super) text: Arc<str>,
    /// Original generation-relevant parser facts.
    pub(super) parsed: ParsedDartFileSurface,
    /// Parser failures retained for source-aware diagnostics.
    pub(super) diagnostics: Vec<Diagnostic>,
    /// Interface failures only; generation must retain the complete diagnostics.
    pub(super) namespace_diagnostics: Vec<Diagnostic>,
    /// Resolved structured directive references, including URI failures.
    pub(super) targets: BTreeMap<String, Result<Source, String>>,
    /// Actually traversed edges used by dependency fingerprints.
    pub(super) dependencies: BTreeSet<String>,
    /// Verified declaring root of an empty generated primary part.
    pub(super) generated_owner: Option<String>,
    /// Whether a standalone marked Dust output was replaced by an empty namespace.
    pub(super) generated_library: bool,
}

/// Read interfaces and imports needed for root metadata or named aliases.
pub(super) struct Loader<'a> {
    /// Read or failed units keyed by canonical library identity.
    pub(super) units: BTreeMap<String, Result<Unit, String>>,
    /// Resolved package and SDK metadata for directive lookup.
    index: &'a SourceIndex,
    /// Deterministically ordered sources awaiting parsing.
    queue: BTreeMap<String, Source>,
    /// Diagnostic IDs already allocated for source identities.
    ids: BTreeMap<String, FileId>,
    /// Libraries whose imports must be traversed for metadata or named aliases.
    imports: BTreeSet<String>,
    /// Declaring libraries for parts whose aliases need their owner's imports.
    part_owners: BTreeMap<String, BTreeSet<String>>,
    /// Generated primary part identities and their declared owners.
    generated: BTreeMap<String, String>,
    /// Configured primary output suffix, not a generic generated-file heuristic.
    primary_suffix: &'a str,
    /// Plugin-declared paths, provisional until handwritten producer verification.
    potential: &'a BTreeSet<std::path::PathBuf>,
    /// Shared parser backend used with each source's effective language version.
    backend: TreeSitterDartBackend,
}

impl<'a> Loader<'a> {
    /// Traverses interfaces and promotes imports required by transparent aliases.
    pub(super) fn load(
        index: &'a SourceIndex,
        roots: &[Source],
        suffix: &'a str,
        potential: &'a BTreeSet<std::path::PathBuf>,
    ) -> Result<Self, String> {
        let mut loader = Self {
            units: BTreeMap::new(),
            index,
            queue: BTreeMap::new(),
            ids: BTreeMap::new(),
            imports: BTreeSet::new(),
            part_owners: BTreeMap::new(),
            generated: BTreeMap::new(),
            primary_suffix: suffix,
            potential,
            backend: TreeSitterDartBackend::new(),
        };
        for root in roots {
            loader.imports.insert(root.uri.clone());
            loader.queue.insert(root.uri.clone(), root.clone());
        }
        loop {
            while let Some((uri, source)) = loader.queue.pop_first() {
                let id = loader.file_id(&uri)?;
                let unit = loader.read(source, id);
                loader.units.insert(uri.clone(), unit);
                loader.expand(&uri)?;
            }
            let mut promote = BTreeSet::new();
            for (uri, unit) in &loader.units {
                let Ok(unit) = unit else {
                    continue;
                };
                if !unit.parsed.typedefs.iter().any(|alias| {
                    alias
                        .parsed_aliased_type
                        .as_ref()
                        .is_some_and(|ty| matches!(ty.kind, ParsedTypeKind::Named { .. }))
                }) {
                    continue;
                }
                if let Some(owners) = loader.part_owners.get(uri) {
                    promote.extend(owners.clone());
                } else {
                    promote.insert(uri.clone());
                }
            }
            let mut promoted = false;
            for uri in promote {
                if loader.imports.insert(uri.clone()) {
                    loader.expand(&uri)?;
                    promoted = true;
                }
            }
            if !promoted {
                break;
            }
        }
        Ok(loader)
    }

    /// Allocates one checked diagnostic ID per source identity.
    fn file_id(&mut self, uri: &str) -> Result<FileId, String> {
        if let Some(id) = self.ids.get(uri) {
            return Ok(*id);
        }
        let id =
            u32::try_from(self.ids.len() + 1).map_err(|_| "too many namespace source files")?;
        let id = FileId::new(id);
        self.ids.insert(uri.to_owned(), id);
        Ok(id)
    }

    /// Reads and parses once, excluding marked Dust annotation provenance.
    fn read(&self, source: Source, id: FileId) -> Result<Unit, String> {
        let owner = self.generated.get(&source.uri).cloned();
        let mut generated_library = false;
        let text = if owner.is_some() {
            String::new()
        } else {
            let text = match fs::read_to_string(&source.path) {
                Ok(text) => text,
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound
                        && self.potential.contains(&source.path) =>
                {
                    generated_library = true;
                    String::new()
                }
                Err(error) => {
                    return Err(format!(
                        "cannot read namespace source `{}`: {error}",
                        source.path.display()
                    ));
                }
            };
            generated_library |= crate::generated_source::is_dust_generated_source(&text);
            if generated_library {
                String::new()
            } else {
                text
            }
        };
        let text: Arc<str> = text.into();
        let parsed = parse_file_with_backend(
            &self.backend,
            &SourceText::new(id, Arc::clone(&text)),
            dust_parser_dart::ParseOptions {
                language_version: source.language_version,
                ..Default::default()
            },
        );
        let mut surface = parsed.library;
        if let Some(owner) = &owner {
            surface.directives.push(ParsedDirective::PartOf {
                uri_literal: Some(ParsedUriLiteral {
                    source: dust_dart_emit::dart_string_literal(owner),
                    span: TextRange::empty(0_u32),
                }),
                library_name: None,
                uri: Some(owner.clone()),
                span: TextRange::empty(0_u32),
            });
        }
        let graph_surface = namespace_surface(&surface, &source.uri);
        let mut targets = BTreeMap::new();
        for directive in &graph_surface.directives {
            for literal in literals(directive) {
                if let Some(reference) = literal.value() {
                    targets.insert(reference.clone(), self.index.resolve(&source, &reference));
                }
            }
        }
        Ok(Unit {
            source,
            file_id: id,
            text,
            parsed: surface,
            diagnostics: parsed.diagnostics,
            namespace_diagnostics: parsed.namespace_diagnostics,
            targets,
            dependencies: BTreeSet::new(),
            generated_owner: owner,
            generated_library,
        })
    }

    /// Queues required interfaces and verifies potential generated primary parts.
    fn expand(&mut self, uri: &str) -> Result<(), String> {
        let Some(Ok(unit)) = self.units.get(uri) else {
            return Ok(());
        };
        let surface = namespace_surface(&unit.parsed, uri);
        let output = dust_workspace::primary_output_path(&unit.source.path, self.primary_suffix);
        let mut dependencies = BTreeSet::new();
        let mut pending = Vec::new();
        for directive in &surface.directives {
            if matches!(directive, ParsedDirective::Import { .. }) && !self.imports.contains(uri) {
                continue;
            }
            if matches!(directive, ParsedDirective::PartOf { .. }) {
                continue;
            }
            for literal in literals(directive) {
                let Some(reference) = literal.value() else {
                    continue;
                };
                let Some(Ok(target)) = unit.targets.get(&reference) else {
                    continue;
                };
                dependencies.insert(target.uri.clone());
                let part = matches!(directive, ParsedDirective::Part { .. });
                let generated = part
                    && target.path == output
                    && match fs::read_to_string(&target.path) {
                        Ok(text) => crate::generated_source::is_dust_generated_source(&text),
                        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
                    };
                pending.push((target.clone(), part, generated));
            }
        }
        self.units
            .get_mut(uri)
            .and_then(|unit| unit.as_mut().ok())
            .expect("existing unit")
            .dependencies
            .extend(dependencies);
        for (target, part, generated) in pending {
            if part {
                self.part_owners
                    .entry(target.uri.clone())
                    .or_default()
                    .insert(uri.to_owned());
            }
            if generated {
                if let Some(previous) = self.generated.insert(target.uri.clone(), uri.to_owned()) {
                    if previous != uri {
                        return Err(format!(
                            "generated part `{}` has conflicting owners",
                            target.uri
                        ));
                    }
                }
            }
            if !self.units.contains_key(&target.uri)
                || (generated
                    && !self
                        .units
                        .get(&target.uri)
                        .and_then(|unit| unit.as_ref().ok())
                        .is_some_and(|unit| unit.generated_owner.is_some()))
            {
                self.queue.insert(target.uri.clone(), target);
            }
        }
        Ok(())
    }
}
