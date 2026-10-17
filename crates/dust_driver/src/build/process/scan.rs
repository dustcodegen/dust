use dust_diagnostics::Diagnostic;
use dust_ir::DartFileIr;
use dust_parser_dart::ParsedDartFileSurface;
use dust_plugin_api::{LibraryAnalysisSnapshot, PluginRegistry, WorkspaceAnalysisBuilder};
use dust_resolver::SymbolCatalog;

use crate::build::work::{available_worker_count, round_robin_groups};

use super::{LoweringConfig, PendingLibrary, execute::resolve_and_lower_library};

/// Workspace facts and preprocessed libraries collected during the scan phase.
pub(crate) struct WorkspaceAnalysisResult {
    /// Plugin facts merged across all pending libraries.
    pub(crate) analysis: WorkspaceAnalysisBuilder,
    /// Parsed surfaces retained for fallback processing.
    pub(crate) pre_parsed: Vec<Option<ParsedDartFileSurface>>,
    /// Parser diagnostics retained for normal processing.
    pub(crate) pre_parse_diagnostics: Vec<Vec<Diagnostic>>,
    /// Lowered IR retained for normal processing and emission.
    pub(crate) pre_lowered: Vec<Option<DartFileIr>>,
    /// Per-library plugin facts persisted into cache entries.
    pub(crate) snapshots: Vec<LibraryAnalysisSnapshot>,
}

/// Collects plugin workspace analysis for pending libraries in parallel.
pub(crate) fn collect_workspace_analysis(
    pending: &[PendingLibrary],
    package_root: &std::path::Path,
    package_name: &str,
    catalog: &SymbolCatalog,
    registry: &PluginRegistry,
    namespaces: &crate::namespace_sources::NamespaceSources,
) -> WorkspaceAnalysisResult {
    if pending.is_empty() {
        return WorkspaceAnalysisResult {
            analysis: WorkspaceAnalysisBuilder::default(),
            pre_parsed: Vec::new(),
            pre_parse_diagnostics: Vec::new(),
            pre_lowered: Vec::new(),
            snapshots: Vec::new(),
        };
    }

    let threads = available_worker_count(pending.len(), None);
    let groups = round_robin_groups(pending.iter().enumerate(), threads);
    let lowering = LoweringConfig {
        package_root,
        package_name,
        catalog,
        registry,
    };
    let lowering = &lowering;

    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(groups.len());
        for group in groups {
            handles.push(scope.spawn(move || {
                let mut local_analysis = WorkspaceAnalysisBuilder::default();
                let mut local_results = Vec::new();

                for (index, pending) in group {
                    let (parsed, parse_diagnostics) = namespaces
                        .parsed(&pending.library.source_path)
                        .expect("discovered namespace root has parser facts");
                    let mut library_analysis = WorkspaceAnalysisBuilder::default();
                    let lowered = lower_for_workspace_analysis(
                        pending.file_id,
                        &pending.library,
                        parsed,
                        parse_diagnostics,
                        lowering,
                        namespaces.scope(&pending.library.source_path),
                    );
                    if let Some(lowered) = lowered.as_ref() {
                        registry.collect_workspace_analysis_ir(lowered, &mut library_analysis);
                    }
                    let analysis_snapshot = library_analysis.snapshot();
                    local_analysis.merge(library_analysis);
                    local_results.push((
                        index,
                        analysis_snapshot,
                        parsed.clone(),
                        parse_diagnostics.to_vec(),
                        lowered,
                    ));
                }

                (local_analysis, local_results)
            }));
        }

        let mut workspace_analysis = WorkspaceAnalysisBuilder::default();
        let mut ordered_surfaces = vec![None; pending.len()];
        let mut ordered_diagnostics = vec![Vec::new(); pending.len()];
        let mut ordered_lowered = vec![None; pending.len()];
        let mut analysis_snapshots = vec![LibraryAnalysisSnapshot::default(); pending.len()];

        for handle in handles {
            let (local_analysis, local) = handle.join().expect("scan thread must not panic");
            workspace_analysis.merge(local_analysis);
            for (index, analysis_snapshot, parsed, diagnostics, lowered) in local {
                analysis_snapshots[index] = analysis_snapshot;
                ordered_surfaces[index] = Some(parsed);
                ordered_diagnostics[index] = diagnostics;
                ordered_lowered[index] = lowered;
            }
        }

        WorkspaceAnalysisResult {
            analysis: workspace_analysis,
            pre_parsed: ordered_surfaces,
            pre_parse_diagnostics: ordered_diagnostics,
            pre_lowered: ordered_lowered,
            snapshots: analysis_snapshots,
        }
    })
}

/// A watch batch still needs current facts from libraries it will not regenerate.
pub(crate) fn collect_remaining_workspace_analysis(
    libraries: &[dust_workspace::SourceLibrary],
    processing: &[dust_workspace::SourceLibrary],
    lowering: &LoweringConfig<'_>,
    namespaces: &crate::namespace_sources::NamespaceSources,
) -> WorkspaceAnalysisBuilder {
    let processing = processing
        .iter()
        .map(|library| &library.source_path)
        .collect::<std::collections::BTreeSet<_>>();
    let mut analysis = WorkspaceAnalysisBuilder::default();
    for library in libraries {
        if processing.contains(&library.source_path) {
            continue;
        }
        let (parsed, diagnostics) = namespaces
            .parsed(&library.source_path)
            .expect("discovered namespace root has parser facts");
        if let Some(lowered) = lower_for_workspace_analysis(
            namespaces
                .file_id(&library.source_path)
                .expect("namespace root id"),
            library,
            parsed,
            diagnostics,
            lowering,
            namespaces.scope(&library.source_path),
        ) {
            lowering
                .registry
                .collect_workspace_analysis_ir(&lowered, &mut analysis);
        }
    }
    analysis
}

/// Lowers a parsed library once so IR-native workspace analysis can feed emission.
fn lower_for_workspace_analysis(
    file_id: dust_text::FileId,
    library: &dust_workspace::SourceLibrary,
    parsed: &ParsedDartFileSurface,
    parse_diagnostics: &[Diagnostic],
    lowering: &LoweringConfig<'_>,
    namespace: Option<dust_resolver::NamespaceScope<'_>>,
) -> Option<DartFileIr> {
    if !parse_diagnostics.is_empty() {
        return None;
    }

    let mut diagnostics = Vec::new();
    let lowered = resolve_and_lower_library(
        file_id,
        library,
        parsed,
        lowering,
        namespace,
        &mut diagnostics,
    )?;
    diagnostics.is_empty().then_some(lowered)
}
