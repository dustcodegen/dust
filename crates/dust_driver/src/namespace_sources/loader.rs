use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs,
    sync::{Arc, OnceLock},
};

use dust_diagnostics::Diagnostic;
use dust_parser_dart::{ParsedDartFileSurface, ParsedDirective};
use dust_text::FileId;

use super::index::{Source, SourceIndex};

/// Graph-only implicit imports and exact directive URI extraction.
mod directives;
/// Demand-driven import expansion for aliases and actual lexical initializers.
mod imports;
/// Snapshot input and parser facts before namespace resolution.
mod input;
/// Bounded preparation of independent parser facts.
mod prepare;
/// Canonical work and the subset eligible for independent parser preparation.
mod queue;
/// Root snapshot preparation overlaps output planning without committing units.
mod roots;
/// Shared immutable directive resolution for local and prepared sources.
mod targets;
/// Assembly of retained source facts and generated primary placeholders.
mod unit;

pub(super) use directives::implicit_core;
use directives::literals;
use input::{ParsePolicy, SourceInput};
use prepare::{KnownSources, ParseAhead};
pub(super) use roots::{PreparedRoots, prepare_roots};
use targets::{ResolvedTargets, resolve_targets};

/// One read source snapshot, shared by graph construction and generation.
pub(super) struct Unit {
    /// Effective identity, physical path and language version.
    pub(super) source: Source,
    /// Stable diagnostic ID within this source graph snapshot.
    pub(super) file_id: FileId,
    /// Exact retained source text, empty for excluded Dust outputs.
    pub(super) text: Arc<str>,
    /// Lazily shared digest of the retained source text.
    pub(super) content_digest: OnceLock<u64>,
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
    /// Processed source snapshots, including replacements and read failures.
    pub(super) revision: usize,
    /// Resolved package and SDK metadata for directive lookup.
    index: &'a SourceIndex,
    /// Deterministically ordered sources awaiting parsing.
    queue: BTreeMap<String, Source>,
    /// Only unowned, uncached sources awaiting independent parser preparation.
    prepare_queue: BTreeMap<String, Source>,
    /// Diagnostic IDs already allocated for source identities.
    ids: HashMap<String, FileId>,
    /// Libraries whose imports must be traversed for metadata or named aliases.
    imports: HashSet<String>,
    /// Declaring libraries for parts whose aliases need their owner's imports.
    part_owners: BTreeMap<String, BTreeSet<String>>,
    /// Generated primary part identities and their declared owners.
    generated: BTreeMap<String, String>,
    /// Configured primary output suffix, not a generic generated-file heuristic.
    primary_suffix: &'a str,
    /// Plugin-declared paths, provisional until handwritten producer verification.
    potential: &'a BTreeSet<std::path::PathBuf>,
    /// Maximum number of source parser workers for this snapshot.
    workers: usize,
    /// Root/external extraction choices shared by coordinator and parser workers.
    parse_policy: Arc<ParsePolicy>,
    /// Preparsed roots are consumed once and never also enter the foreign pool.
    prepared_roots: PreparedRoots,
}

impl<'a> Loader<'a> {
    /// Traverses interfaces and promotes imports required by transparent aliases.
    pub(super) fn load(
        index: &'a SourceIndex,
        roots: &[Source],
        suffix: &'a str,
        potential: &'a BTreeSet<std::path::PathBuf>,
        workers: usize,
        prepared_roots: PreparedRoots,
    ) -> Result<Self, String> {
        let mut loader = Self {
            units: BTreeMap::new(),
            revision: 0,
            index,
            queue: BTreeMap::new(),
            prepare_queue: BTreeMap::new(),
            ids: HashMap::new(),
            imports: HashSet::new(),
            part_owners: BTreeMap::new(),
            generated: BTreeMap::new(),
            primary_suffix: suffix,
            potential,
            workers,
            parse_policy: Arc::clone(&prepared_roots.policy),
            prepared_roots,
        };
        for root in roots {
            loader.imports.insert(root.uri.clone());
            loader.enqueue(root.clone());
        }
        loader.drain()?;
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
    fn read(
        &mut self,
        source: Source,
        id: FileId,
        ahead: Option<&mut ParseAhead>,
    ) -> Result<Unit, String> {
        let owner = self.generated.get(&source.uri).cloned();
        let snapshot = self.parse_policy.preloaded_input(&source);
        let input = SourceInput::read_with_snapshot(
            &source,
            owner,
            self.potential,
            snapshot.as_ref().map(|input| &input.text),
        );
        let cached = self.prepared_roots.contains(&source.uri);
        let prepared = if cached {
            self.prepared_roots.take(&source, input.as_ref().ok(), id)
        } else {
            ahead.and_then(|ahead| {
                ahead.take(
                    &source,
                    input.as_ref().ok(),
                    id,
                    &self.prepare_queue,
                    &self.generated,
                )
            })
        };
        let input = input?;
        let (parsed, targets) = match prepared {
            Some((parsed, targets)) => (parsed, Some(targets)),
            None => {
                let result = input.parse(&source, id, &self.parse_policy);
                (result, None)
            }
        };
        let result = self.resolve(source, id, input, parsed, targets);
        Ok(result)
    }

    /// Queues required interfaces and verifies potential generated primary parts.
    fn expand(&mut self, uri: &str) -> Result<(), String> {
        let Some(Ok(unit)) = self.units.get(uri) else {
            return Ok(());
        };
        let core = implicit_core(&unit.parsed.directives, uri);
        let mut output = None;
        let mut dependencies = BTreeSet::new();
        let mut pending = Vec::new();
        for directive in unit.parsed.directives.iter().chain(core.as_ref()) {
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
                    && target.path
                        == *output.get_or_insert_with(|| {
                            dust_workspace::primary_output_path(
                                &unit.source.path,
                                self.primary_suffix,
                            )
                        })
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
                self.prepare_queue.remove(&target.uri);
                if let Some(previous) = self.generated.insert(target.uri.clone(), uri.to_owned())
                    && previous != uri
                {
                    return Err(format!(
                        "generated part `{}` has conflicting owners",
                        target.uri
                    ));
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
                self.enqueue(target);
            }
        }
        Ok(())
    }
}
