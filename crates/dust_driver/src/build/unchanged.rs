use std::{collections::BTreeMap, path::Path, sync::atomic::AtomicUsize};

use dust_cache::WorkspaceCache;
use dust_workspace::{WorkspacePlan, load_workspace};

use crate::{
    compatibility::validate_workspace_package_versions,
    namespace_sources::NamespaceSources,
    request::BuildRequest,
    result::{CacheReport, CommandResult},
};

use super::{
    RegistrySelection,
    batch::{ProgressCallback, ProgressReporter, ProgressSnapshot},
    codegen_tool_hash_for_selection,
    process::OutputSnapshots,
    read_workspace_config_hash,
};

/// Exact file contents and output-set validation.
mod input;
/// Disposable proof of a complete, diagnostic-free build.
mod manifest;

use manifest::Manifest;

/// Reuses a successful build only while every observed input and output is unchanged.
pub(crate) fn try_build(
    request: &BuildRequest,
    progress: Option<&ProgressCallback<'_>>,
) -> Option<CommandResult> {
    let mut workspace = load_workspace(&request.cwd).ok()?;
    eligible(request, &workspace).then_some(())?;
    let tool = codegen_tool_hash_for_selection(RegistrySelection::for_build(request.db));
    let cache_hash = input::read_cache_hash(&input::cache_path(&workspace)).ok()??;
    let manifest = Manifest::load_matching(&workspace.cache_root, tool.value(), cache_hash)?;
    if manifest.metadata
        != NamespaceSources::metadata_snapshot(
            &request.cwd,
            &workspace,
            request.sdk_source_root.as_deref(),
        )
        .ok()?
        || manifest.lib_files != input::lib_files(&workspace.package_root.join("lib"))?
        || manifest
            .lib_files
            .iter()
            .any(|path| !manifest.files.contains_key(path))
        || manifest
            .metadata
            .iter()
            .any(|(path, hash)| manifest.files.get(path) != Some(&Some(*hash)))
        || manifest
            .artifacts
            .iter()
            .any(|(source, output, auxiliary, _, _)| {
                !manifest.files.contains_key(source)
                    || !manifest.files.contains_key(output)
                    || auxiliary
                        .iter()
                        .any(|path| !manifest.files.contains_key(path))
            })
        || !input::matches(&manifest.files)
    {
        return None;
    }
    workspace.dust_packages = manifest.dust_packages;
    validate_workspace_package_versions(&workspace).ok()?;
    let artifacts = manifest
        .artifacts
        .into_iter()
        .map(manifest::artifact)
        .collect::<Vec<_>>();
    let completed = AtomicUsize::new(0);
    let reporter = ProgressReporter::new(
        progress,
        &completed,
        crate::ProgressPhase::Build,
        artifacts.len(),
    );
    reporter.started_batch();
    for artifact in &artifacts {
        reporter.finish(ProgressSnapshot {
            library: &dust_workspace::SourceLibrary {
                source_path: artifact.source_path.clone(),
                output_path: artifact.output_path.clone(),
            },
            cached: true,
            routed: artifact.routed,
            written: false,
            changed: false,
            had_errors: false,
            elapsed_ms: 0,
        });
    }
    Some(CommandResult {
        cache: Some(CacheReport {
            path: input::cache_path(&workspace),
            hits: artifacts.len(),
            misses: 0,
        }),
        build_artifacts: artifacts,
        ..Default::default()
    })
}

/// Records original input digests only after outputs and cache metadata are verified.
pub(crate) fn save(
    request: &BuildRequest,
    workspace: &WorkspacePlan,
    namespaces: &NamespaceSources,
    cache: &WorkspaceCache,
    result: &CommandResult,
    fresh: &OutputSnapshots,
) {
    if let Some(manifest) = prepare(request, workspace, namespaces, cache, result, fresh) {
        manifest.save(&workspace.cache_root);
    }
}

/// Produces no proof for unsupported external inputs, diagnostics or changing snapshots.
fn prepare(
    request: &BuildRequest,
    workspace: &WorkspacePlan,
    namespaces: &NamespaceSources,
    cache: &WorkspaceCache,
    result: &CommandResult,
    fresh: &OutputSnapshots,
) -> Option<Manifest> {
    if !eligible(request, workspace) || !result.diagnostics.is_empty() {
        return None;
    }
    let mut previous = workspace.clone();
    previous.libraries.clear();
    previous.dust_packages.clear();
    if load_workspace(&request.cwd).ok()? != previous {
        return None;
    }
    let inputs = namespaces.input_snapshot()?;
    let metadata = NamespaceSources::metadata_snapshot(
        &request.cwd,
        workspace,
        request.sdk_source_root.as_deref(),
    )
    .ok()?;
    if metadata
        .iter()
        .any(|(path, hash)| inputs.get(path) != Some(hash))
    {
        return None;
    }
    let tool = codegen_tool_hash_for_selection(RegistrySelection::for_build(request.db));
    let mut configs = input::package_specs(workspace)?;
    for path in [
        workspace.package_config.path.clone(),
        workspace.package_root.join("pubspec.yaml"),
        workspace.package_root.join("dust.yaml"),
    ] {
        configs.insert(path.clone(), input::read_hash(&path).ok()?);
    }
    let config = read_workspace_config_hash(
        &workspace.package_config.path,
        workspace.dust_config.path.as_deref(),
        &workspace.package_root.join("pubspec.yaml"),
    )
    .ok()?;
    let mut files = inputs
        .into_iter()
        .map(|(path, hash)| (path, Some(hash)))
        .collect::<BTreeMap<_, _>>();
    let previous = result
        .cache
        .as_ref()
        .filter(|report| report.hits != 0)
        .and_then(|_| Manifest::load(&workspace.cache_root));
    let outputs = input::output_snapshot(
        workspace,
        cache,
        &result.build_artifacts,
        tool.value(),
        config,
        input::OutputEvidence {
            fresh,
            previous: previous.as_ref(),
        },
    )?;
    let mut lib_files = namespaces
        .discovered_root_paths()
        .map(Path::to_path_buf)
        .chain(
            outputs
                .iter()
                .filter_map(|(path, hash)| hash.map(|_| path.clone())),
        )
        .collect::<Vec<_>>();
    files = input::merge(files, outputs)?;
    for path in namespaces.generated_paths() {
        let hash = match files.get(&path) {
            Some(hash) => *hash,
            None => {
                let hash = input::generated_hash(&path)?;
                files.insert(path.clone(), hash);
                hash
            }
        };
        if hash.is_some() {
            lib_files.push(path);
        }
    }
    for (path, hash) in &configs {
        if files
            .insert(path.clone(), *hash)
            .is_some_and(|before| before != *hash)
        {
            return None;
        }
    }
    let library_root = workspace.package_root.join("lib");
    lib_files.retain(|path| {
        path.starts_with(&library_root)
            && path
                .extension()
                .is_some_and(|extension| extension == "dart")
    });
    lib_files.sort();
    lib_files.dedup();
    if lib_files.iter().any(|path| !files.contains_key(path)) {
        return None;
    }
    if configs
        .iter()
        .any(|(path, hash)| input::read_hash(path).ok() != Some(*hash))
    {
        return None;
    }
    Some(Manifest {
        integrity: 0,
        schema: 3,
        tool_hash: tool.value(),
        cache_hash: cache.persisted_hash()?,
        metadata,
        lib_files,
        files,
        dust_packages: workspace.dust_packages.clone(),
        artifacts: result
            .build_artifacts
            .iter()
            .map(|artifact| {
                Some((
                    artifact.source_path.clone(),
                    artifact.output_path.clone(),
                    artifact.auxiliary_output_paths.clone(),
                    artifact.routed,
                    cache
                        .get(&workspace.cache_root, &artifact.source_path)?
                        .expected_output_hash,
                ))
            })
            .collect::<Option<Vec<_>>>()?,
    })
}

/// Database, server destinations and i18n retain their complete command pipeline.
fn eligible(request: &BuildRequest, workspace: &WorkspacePlan) -> bool {
    !request.db.only_db
        && !request.db.offline
        && workspace.dust_config.server.is_none()
        && workspace.dust_config.i18n.is_none()
}

#[cfg(test)]
mod tests;
