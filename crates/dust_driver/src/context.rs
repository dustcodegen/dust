use std::path::Path;

use dust_cache::WorkspaceCache;
use dust_diagnostics::Diagnostic;
use dust_plugin_api::PluginRegistry;
use dust_resolver::SymbolCatalog;
use dust_workspace::{SupportedAnnotations, WorkspacePlan, discover_workspace};

use crate::{
    build::{
        CodegenToolHash, RegistrySelection, codegen_tool_hash_for_selection,
        read_workspace_config_hash, registry_for_selection,
    },
    catalog::build_symbol_catalog,
    compatibility::validate_workspace_package_versions,
    namespace_sources::NamespaceSources,
    result::CacheReport,
};

/// Workspace context shared by commands that do not need cache metadata.
pub(crate) struct DriverContext {
    /// Discovered workspace plan.
    pub(crate) workspace: WorkspacePlan,
    /// Plugin registry selected for the command.
    pub(crate) registry: PluginRegistry,
    /// Resolver catalog derived from the registry.
    pub(crate) catalog: SymbolCatalog,
}

impl DriverContext {
    /// Discovers the workspace and builds the selected registry and catalog.
    pub(crate) fn load(cwd: &Path, selection: RegistrySelection) -> Result<Self, Diagnostic> {
        let discovery_registry = registry_for_selection(selection);
        let supported_annotations: SupportedAnnotations = discovery_registry
            .all_supported_annotations()
            .into_iter()
            .collect();
        let supported_annotations = match selection {
            RegistrySelection::All => supported_annotations,
            RegistrySelection::ValidatingPlugin { .. } => {
                supported_annotations.with_unannotated_libraries()
            }
        };
        let workspace = discover_workspace(cwd, &supported_annotations)?;
        let destination = crate::server_client::resolve_destination(&workspace)?;
        let registry =
            crate::build::registry_for_selection_with_server_client(selection, destination);
        let catalog = build_symbol_catalog(&registry)?;

        Ok(Self {
            workspace,
            registry,
            catalog,
        })
    }
}

/// Workspace context shared by commands that need cache metadata.
pub(crate) struct CachedDriverContext {
    /// Discovered workspace plan.
    pub(crate) workspace: WorkspacePlan,
    /// Plugin registry selected for the command.
    pub(crate) registry: PluginRegistry,
    /// Resolver catalog derived from the registry.
    pub(crate) catalog: SymbolCatalog,
    /// Actual source snapshot used consistently for discovery and processing.
    pub(crate) namespaces: NamespaceSources,
    /// Hash of active code generation logic and plugins.
    pub(crate) tool_hash: CodegenToolHash,
    /// Hash of package and Dust configuration files.
    pub(crate) package_config_hash: u64,
    /// Mutable workspace cache.
    pub(crate) cache: WorkspaceCache,
    /// Cache report initialized with the cache storage path.
    pub(crate) cache_report: CacheReport,
}

impl CachedDriverContext {
    /// Loads workspace, registry, catalog, hashes, and cache state.
    pub(crate) fn load(
        cwd: &Path,
        selection: RegistrySelection,
        sdk_root: Option<&Path>,
    ) -> Result<Self, Diagnostic> {
        let DriverContext {
            mut workspace,
            registry,
            catalog,
        } = DriverContext::load(cwd, selection)?;
        if !workspace.package_root.join("lib").exists() {
            validate_workspace_libraries(&workspace)?;
        }
        validate_workspace_package_versions(&workspace)?;
        let namespaces = NamespaceSources::load(
            cwd,
            &mut workspace,
            sdk_root,
            &catalog,
            &registry,
            !matches!(selection, RegistrySelection::All),
        )?;
        validate_workspace_libraries(&workspace)?;
        validate_workspace_package_versions(&workspace)?;
        let tool_hash = codegen_tool_hash_for_selection(selection);
        let package_config_hash = read_workspace_config_hash(
            &workspace.package_config.path,
            workspace.dust_config.path.as_deref(),
            &workspace.package_root.join("pubspec.yaml"),
        )?;
        let cache = load_workspace_cache(&workspace)?;
        let cache_report = CacheReport {
            path: cache.path().to_path_buf(),
            ..CacheReport::default()
        };

        Ok(Self {
            workspace,
            registry,
            catalog,
            namespaces,
            tool_hash,
            package_config_hash,
            cache,
            cache_report,
        })
    }
}

/// Rejects commands that would otherwise succeed without checking any Dart libraries.
fn validate_workspace_libraries(workspace: &WorkspacePlan) -> Result<(), Diagnostic> {
    if workspace.is_pub_workspace_root && workspace.libraries.is_empty() {
        return Err(Diagnostic::error(
            "no Dart libraries found; run Dust inside a workspace member or pass --root <member>",
        ));
    }

    Ok(())
}

/// Loads the workspace cache and converts IO failures into diagnostics.
fn load_workspace_cache(workspace: &WorkspacePlan) -> Result<WorkspaceCache, Diagnostic> {
    WorkspaceCache::load(&workspace.cache_root).map_err(|error| {
        Diagnostic::error(format!(
            "failed to load Dust cache `{}`: {error}",
            workspace
                .cache_root
                .join(".dart_tool/dust/build_cache_v1.json")
                .display()
        ))
    })
}
