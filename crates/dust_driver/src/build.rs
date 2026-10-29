/// Applies per-library build outcomes to command results and cache state.
mod apply;
/// Batch loading, cache selection, workspace analysis, and parallel processing.
mod batch;
/// Per-library parse, resolve, lower, and emit pipeline.
mod process;
/// Registry, hashing, and cache input helpers.
mod support;
/// Worker-count and work-distribution helpers.
mod work;

use std::time::Instant;

use dust_workspace::{detect_workspace_root, load_dust_config};

use crate::{
    context::CachedDriverContext,
    i18n_bootstrap::build_i18n_bootstrap,
    progress::{ProgressEvent, ProgressPhase},
    request::BuildRequest,
    result::CommandResult,
};

pub(crate) use apply::{ApplyOutcomeConfig, apply_indexed_outcomes, flush_cache_into_result};
pub(crate) use batch::{BatchConfig, prepare_and_process_batch};
pub(crate) use support::{
    CodegenToolHash, RegistrySelection, codegen_tool_hash_for_selection, hash_text,
    read_workspace_config_hash, registry_for_selection, registry_for_selection_with_server_clients,
};

/// Runs one writing build across the discovered workspace.
pub fn run_build(request: BuildRequest) -> CommandResult {
    run_build_inner(request, None)
}

/// Runs one writing build across the discovered workspace while emitting progress events.
pub fn run_build_with_progress<F>(request: BuildRequest, progress: F) -> CommandResult
where
    F: Fn(ProgressEvent) + Send + Sync,
{
    run_build_inner(request, Some(&progress))
}

/// Shared implementation for build commands with optional progress reporting.
fn run_build_inner(
    request: BuildRequest,
    progress: Option<&(dyn Fn(ProgressEvent) + Send + Sync + '_)>,
) -> CommandResult {
    let started = Instant::now();
    let mut result = CommandResult::default();
    let i18n_bootstrap = match detect_workspace_root(&request.cwd).and_then(|root| {
        let config = load_dust_config(&root)?;
        build_i18n_bootstrap(&root, &config)
    }) {
        Ok(artifact) => artifact,
        Err(diagnostic) => {
            result.diagnostics.push(diagnostic);
            result.elapsed_ms = started.elapsed().as_millis();
            return result;
        }
    };

    let CachedDriverContext {
        workspace,
        registry,
        catalog,
        namespaces,
        tool_hash,
        package_config_hash,
        mut cache,
        mut cache_report,
    } = match CachedDriverContext::load(
        &request.cwd,
        RegistrySelection::for_build(request.db),
        request.sdk_source_root.as_deref(),
    ) {
        Ok(context) => context,
        Err(diagnostic) => {
            result.diagnostics.push(diagnostic);
            result.elapsed_ms = started.elapsed().as_millis();
            return result;
        }
    };
    let batch = prepare_and_process_batch(
        BatchConfig {
            analysis_libraries: &workspace.libraries,
            namespaces: &namespaces,
            cache_root: &workspace.cache_root,
            package_root: &workspace.package_root,
            package_name: &workspace.package_name,
            is_flutter_package: workspace.is_flutter_package,
            package_config_hash,
            tool_hash,
            cache: &cache,
            catalog: &catalog,
            registry: &registry,
            write_output: true,
            fail_fast: request.fail_fast,
            jobs: request.jobs,
            phase: ProgressPhase::Build,
            progress,
        },
        &workspace.libraries,
        &mut cache_report,
    );

    result.diagnostic_files.extend(batch.diagnostic_files);
    apply_indexed_outcomes(
        batch.outcomes,
        ApplyOutcomeConfig {
            cache_root: &workspace.cache_root,
            package_config_hash,
            workspace_analysis_hash: batch.workspace_analysis_hash,
            fail_fast: request.fail_fast,
        },
        &mut cache,
        &mut result,
        None,
    );
    if let Some(artifact) = i18n_bootstrap {
        result.build_artifacts.push(artifact);
    }
    flush_cache_into_result(&mut cache, &mut result);
    result.cache = Some(cache_report);
    result.elapsed_ms = started.elapsed().as_millis();
    result
}
