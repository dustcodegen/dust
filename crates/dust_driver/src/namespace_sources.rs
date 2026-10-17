use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use dust_diagnostics::Diagnostic;
use dust_parser_dart::ParsedDartFileSurface;
use dust_resolver::{NamespaceBinding, NamespaceGraph, NamespaceScope, SymbolCatalog};
use dust_text::FileId;
use dust_workspace::WorkspacePlan;

use crate::{build::hash_text, result::DiagnosticFile};

/// Root discovery and ownership preflight.
mod discovery;
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
/// Source-preserving primary-part decisions for the resolver.
mod primary_part;
/// Active SDK discovery and target-independent source maps.
mod sdk;
/// URI identity normalization, separate from filesystem paths.
mod uri;

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
    /// Package configuration and SDK source-map fingerprint.
    metadata_hash: u64,
    /// Fatal ownership diagnostics to reject before any output mutation.
    binding_errors: BTreeMap<PathBuf, Vec<Diagnostic>>,
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
    ) -> Result<Self, Diagnostic> {
        let index = index::SourceIndex::load(cwd, &workspace.package_config.path, sdk_root)
            .map_err(Diagnostic::error)?;
        let mut files = Vec::new();
        discovery::collect_sources(&workspace.package_root.join("lib"), &mut files)?;
        files.sort();
        let roots = files
            .iter()
            .map(|path| index.file(path))
            .collect::<Result<Vec<_>, _>>()
            .map_err(Diagnostic::error)?;
        let claims = outputs::plan(&roots, registry)?;
        let potential = claims.keys().cloned().collect();
        let primary_suffix = workspace.dust_config.outputs.primary_suffix.clone();
        let mut loader = loader::Loader::load(&index, &roots, &primary_suffix, &potential)
            .map_err(Diagnostic::error)?;
        for root in &roots {
            if let Some(Err(error)) = loader.units.get(&root.uri) {
                return Err(Diagnostic::error(error));
            }
        }
        let mut metadata = String::new();
        for (path, text) in &index.inputs {
            metadata.push_str(&path.to_string_lossy());
            metadata.push('\0');
            metadata.push_str(text);
            metadata.push('\0');
        }
        loop {
            let graph = outputs::verify(&mut loader.units, &claims, catalog)?;
            let mut sources = Self {
                graph,
                units: std::mem::take(&mut loader.units),
                roots: roots
                    .iter()
                    .map(|root| (root.path.clone(), root.uri.clone()))
                    .collect(),
                metadata_hash: hash_text(&metadata),
                binding_errors: BTreeMap::new(),
            };
            discovery::select_libraries(&mut sources, workspace, catalog, include_unannotated);
            let required = sources.graph.required_lexical_imports();
            loader.units = std::mem::take(&mut sources.units);
            if loader
                .require_lexical_imports(&required)
                .map_err(Diagnostic::error)?
            {
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

    /// Shares the exact text used for parsing and diagnostic ranges.
    pub(crate) fn source(&self, path: &Path) -> Option<Arc<str>> {
        Some(Arc::clone(&self.unit(path)?.text))
    }

    /// Hashes metadata, dependency texts, URI identities and diagnostic file IDs.
    pub(crate) fn fingerprint(&self, path: &Path) -> u64 {
        let mut text = format!("{}\0", self.metadata_hash);
        if let Some(uri) = self.roots.get(path) {
            for uri in self.read_set(uri) {
                text.push_str(&uri);
                text.push('\0');
                match self.units.get(&uri) {
                    Some(Ok(unit)) => {
                        text.push_str(&unit.file_id.raw().to_string());
                        text.push('\0');
                        text.push_str(&unit.text);
                        if let Some(owner) = &unit.generated_owner {
                            text.push_str(owner);
                        }
                    }
                    Some(Err(error)) => text.push_str(error),
                    None => text.push_str("missing namespace source"),
                }
                text.push('\0');
            }
        }
        hash_text(&text)
    }

    /// Supplies contexts for dependency labels without exposing generated contents.
    pub(crate) fn diagnostic_files(&self) -> Vec<DiagnosticFile> {
        self.units
            .values()
            .filter_map(|unit| unit.as_ref().ok())
            .filter(|unit| unit.generated_owner.is_none() && !unit.generated_library)
            .map(|unit| {
                DiagnosticFile::new(
                    unit.file_id,
                    unit.source.path.clone(),
                    Arc::clone(&unit.text),
                )
            })
            .collect()
    }

    /// Collects the root's transitive, actually traversed source dependencies.
    fn read_set(&self, root: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut queue = BTreeSet::from([root.to_owned()]);
        while let Some(uri) = queue.pop_first() {
            if !seen.insert(uri.clone()) {
                continue;
            }
            if let Some(Ok(unit)) = self.units.get(&uri) {
                queue.extend(unit.dependencies.clone());
            }
        }
        seen
    }
}
