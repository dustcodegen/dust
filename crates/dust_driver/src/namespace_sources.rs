use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use dust_diagnostics::Diagnostic;
use dust_parser_dart::ParsedDartFileSurface;
use dust_resolver::{NamespaceBinding, NamespaceGraph, NamespaceScope, SymbolCatalog};
use dust_text::FileId;
use dust_workspace::WorkspacePlan;

use crate::build::hash_text;

/// Imported annotated libraries available to structured plugin analysis.
mod analysis;
/// Diagnostic sources retained from the canonical graph snapshot.
mod diagnostics;
/// Root discovery and ownership preflight.
mod discovery;
/// Transitive source fingerprints with shared content digests.
mod fingerprint;
/// Shared provisional/final graph construction.
mod graph;
/// Package and SDK import identities and physical source locations.
mod index;
/// Demand-driven parsing of source interfaces and required imports.
mod loader;
/// Metadata sites represented by the current parser surface.
mod metadata;
/// Verification of standalone generated library producers.
mod outputs;
/// Optional source-exact parser facts, independent of code generation cache entries.
pub(crate) mod parser_cache;
/// Source-preserving primary-part decisions for the resolver.
mod primary_part;
/// Active SDK discovery and target-independent source maps.
mod sdk;
/// Exact source snapshots for unchanged-build proofs.
mod snapshot;
/// URI identity normalization, separate from filesystem paths.
mod uri;

/// Shared source-only package and SDK fixtures for driver unit tests.
#[cfg(test)]
#[path = "../../../fixtures/namespace_sources/workspace.rs"]
pub(crate) mod test_workspace;

/// Real public server exports preserve ownership without enabling server registration.
#[cfg(test)]
#[path = "namespace_sources/server_contract_tests.rs"]
mod server_contract_tests;

/// One immutable source graph snapshot for discovery, analysis and output.
pub(crate) struct NamespaceSources {
    /// Resolved namespaces bound to actual declaring-library identities.
    graph: NamespaceGraph,
    /// Immutable parsed sources and retained read failures, keyed by URI.
    units: BTreeMap<String, Result<loader::Unit, String>>,
    /// Workspace entrypoints and their file URI identities.
    roots: BTreeMap<PathBuf, String>,
    /// Shared URI ranks, dependency edges and lazy exact fingerprint records.
    fingerprint_index: OnceLock<fingerprint::Index>,
    /// Package configuration and SDK source-map fingerprint.
    metadata_hash: u64,
    /// Exact package and SDK metadata snapshot.
    metadata_inputs: BTreeMap<PathBuf, u64>,
    /// Native parsing requested a replacement for the optional parser snapshot.
    pub(crate) parser_facts_changed: bool,
    /// Fatal ownership diagnostics to reject before any output mutation.
    binding_errors: BTreeMap<PathBuf, Vec<Diagnostic>>,
    /// Generated primary suffix used for dependency-library identities.
    primary_suffix: String,
}

impl NamespaceSources {
    /// Loads actual source facts, constructs namespaces, and selects codegen roots.
    pub(crate) fn load(
        cwd: &Path,
        workspace: &mut WorkspacePlan,
        sdk_root: Option<&Path>,
        catalog: &SymbolCatalog,
        registry: &dust_plugin_api::PluginRegistry,
        include_unannotated: bool,
        jobs: Option<usize>,
    ) -> Result<Self, Diagnostic> {
        let index = index::SourceIndex::load(
            cwd,
            &workspace.package_config.path,
            sdk_root,
            workspace.is_flutter_package,
        )
        .map_err(Diagnostic::error)?;
        let supported_annotations = registry.all_supported_annotations();
        let inputs = discovery::read_inputs(
            &workspace.package_root.join("lib"),
            &workspace.cache_root,
            &supported_annotations,
        )?;
        let files = inputs.sources;
        let parser_cache = inputs.parser_cache;
        let roots = discovery::index_sources(&index, &files)?;
        let available = crate::build::available_worker_count(usize::MAX, None);
        let workers = jobs.unwrap_or(available).clamp(1, available);
        let (claims, prepared) = loader::prepare_roots(
            &index,
            &roots,
            workers,
            &supported_annotations,
            &files,
            Some(Arc::clone(&parser_cache)),
            || outputs::plan(&roots, registry),
        );
        let claims = claims?;
        let potential = claims.keys().cloned().collect();
        let primary_suffix = workspace.dust_config.outputs.primary_suffix.clone();
        let mut loader = loader::Loader::load(
            &index,
            &roots,
            &primary_suffix,
            &potential,
            workers,
            prepared,
        )
        .map_err(Diagnostic::error)?;
        for root in &roots {
            if let Some(Err(error)) = loader.units.get(&root.uri) {
                return Err(Diagnostic::error(error));
            }
        }
        let supported = registry
            .all_supported_annotations()
            .into_iter()
            .collect::<BTreeSet<_>>();
        let root_uris = roots
            .iter()
            .map(|root| root.uri.as_str())
            .collect::<BTreeSet<_>>();
        let annotated_dependencies = loader
            .units
            .iter()
            .filter_map(|(uri, unit)| {
                let unit = unit.as_ref().ok()?;
                (!root_uris.contains(uri.as_str())
                    && metadata::annotations(&unit.parsed)
                        .iter()
                        .any(|annotation| supported.contains(annotation.name.as_str())))
                .then_some(uri.clone())
            })
            .collect();
        loader
            .require_lexical_imports(&annotated_dependencies)
            .map_err(Diagnostic::error)?;
        let mut metadata = String::new();
        for (path, text) in &index.inputs {
            metadata.push_str(&path.to_string_lossy());
            metadata.push('\0');
            metadata.push_str(text);
            metadata.push('\0');
        }
        let mut reusable_graph = None;
        loop {
            let graph = match reusable_graph.take() {
                Some(graph) => graph,
                None => {
                    let mut units = std::mem::take(&mut loader.units);
                    let verified =
                        outputs::verify_ready(&mut units, &claims, catalog, |required| {
                            loader.imports_ready(required)
                        });
                    loader.units = units;
                    match verified? {
                        outputs::Verification::Ready(graph) => graph,
                        outputs::Verification::NeedsImports(graph) => {
                            loader
                                .require_lexical_imports(&graph.required_lexical_imports())
                                .map_err(Diagnostic::error)?;
                            continue;
                        }
                    }
                }
            };
            let mut sources = Self {
                graph,
                units: std::mem::take(&mut loader.units),
                roots: roots
                    .iter()
                    .map(|root| (root.path.clone(), root.uri.clone()))
                    .collect(),
                fingerprint_index: OnceLock::new(),
                metadata_hash: hash_text(&metadata),
                metadata_inputs: index
                    .inputs
                    .iter()
                    .map(|(path, text)| (path.clone(), hash_text(text)))
                    .collect(),
                parser_facts_changed: parser_cache.needs_save(),
                binding_errors: BTreeMap::new(),
                primary_suffix: primary_suffix.clone(),
            };
            let previous_awareness =
                discovery::select_libraries(&mut sources, workspace, catalog, include_unannotated);
            let required = sources.graph.required_lexical_imports();
            loader.units = std::mem::take(&mut sources.units);
            let before_revision = loader.revision;
            if loader
                .require_lexical_imports(&required)
                .map_err(Diagnostic::error)?
            {
                if loader.revision == before_revision {
                    sources.units = std::mem::take(&mut loader.units);
                    let unchanged = {
                        let awareness = discovery::dust_aware_uris(&sources, catalog);
                        awareness.len() == previous_awareness.len()
                            && awareness
                                .iter()
                                .all(|uri| previous_awareness.contains(*uri))
                    };
                    if unchanged {
                        return Ok(sources);
                    }
                    loader.units = std::mem::take(&mut sources.units);
                    reusable_graph = Some(sources.graph);
                }
                continue;
            }
            sources.units = std::mem::take(&mut loader.units);
            return Ok(sources);
        }
    }

    /// Looks up the retained source unit for one workspace entrypoint.
    fn unit(&self, path: &Path) -> Option<&loader::Unit> {
        self.units
            .get(self.roots.get(path)?)
            .and_then(|unit| unit.as_ref().ok())
    }

    /// Shares the root's verified namespace with analysis and lowering.
    pub(crate) fn scope(&self, path: &Path) -> Option<NamespaceScope<'_>> {
        Some(self.graph.scope(self.roots.get(path)?))
    }

    /// Tests canonical ownership, not the annotation's short spelling.
    pub(crate) fn has_annotation(
        &self,
        path: &Path,
        symbol: &str,
        catalog: &SymbolCatalog,
    ) -> bool {
        let Some(unit) = self.unit(path) else {
            return false;
        };
        let Some(scope) = self.scope(path) else {
            return false;
        };
        metadata::annotations(&unit.parsed).into_iter().any(|annotation| {
            let name = annotation.prefix.as_ref().map_or_else(|| annotation.name.clone(),
                |prefix| format!("{prefix}.{}", annotation.name));
            matches!(scope.bind_at(&name, catalog, dust_ir::SpanIr::new(unit.file_id, annotation.span)), NamespaceBinding::Owned(owner) if owner.0 == symbol)
        })
    }

    /// Returns the diagnostic identity allocated for a root's source snapshot.
    pub(crate) fn file_id(&self, path: &Path) -> Option<FileId> {
        Some(self.unit(path)?.file_id)
    }

    /// Returns root ownership failures collected before generation can start.
    pub(crate) fn binding_errors(&self, path: &Path) -> &[Diagnostic] {
        self.binding_errors.get(path).map_or(&[], Vec::as_slice)
    }

    /// Shares original parser facts and diagnostics without reading source again.
    pub(crate) fn parsed(&self, path: &Path) -> Option<(&ParsedDartFileSurface, &[Diagnostic])> {
        let unit = self.unit(path)?;
        Some((&unit.parsed, &unit.diagnostics))
    }

    /// Reference traversal used to prove exact fingerprint compatibility.
    #[cfg(test)]
    fn read_set<'a>(&'a self, root: &'a str) -> BTreeSet<&'a str> {
        let mut seen = BTreeSet::from([root]);
        let mut pending = vec![root];
        while let Some(uri) = pending.pop() {
            if let Some(Ok(unit)) = self.units.get(uri) {
                pending.extend(
                    unit.dependencies
                        .iter()
                        .map(String::as_str)
                        .filter(|uri| seen.insert(*uri)),
                );
            }
        }
        seen
    }
}
