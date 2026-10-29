use std::{fs, path::Path};

use dust_diagnostics::Diagnostic;
use dust_server_plugin::ServerClientDestination;
use dust_workspace::{PackageConfigKind, ServerClientConfig, WorkspacePlan, load_package_name};
use serde_json::Value;

#[cfg(test)]
mod tests;

/// Resolves the configured generated-client destination inside the active pub workspace.
pub(crate) fn resolve_destinations(
    workspace: &WorkspacePlan,
) -> Result<Vec<ServerClientDestination>, Diagnostic> {
    let Some(config) = &workspace.dust_config.server else {
        return Ok(Vec::new());
    };
    let contents = fs::read_to_string(&workspace.package_config.path).map_err(|error| {
        Diagnostic::error(format!(
            "failed to read package configuration `{}`: {error}",
            workspace.package_config.path.display()
        ))
    })?;
    let value: Value = serde_json::from_str(&contents).map_err(|error| {
        Diagnostic::error(format!(
            "failed to parse package configuration `{}`: {error}",
            workspace.package_config.path.display()
        ))
    })?;
    let entries = value
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| Diagnostic::error("package configuration must declare a packages list"))?;
    config
        .clients
        .iter()
        .map(|client| resolve_destination(workspace, entries, client))
        .collect()
}

/// Resolves and validates one configured destination package.
fn resolve_destination(
    workspace: &WorkspacePlan,
    entries: &[Value],
    config: &ServerClientConfig,
) -> Result<ServerClientDestination, Diagnostic> {
    let matches = entries
        .iter()
        .filter(|entry| entry.get("name").and_then(Value::as_str) == Some(&config.package))
        .collect::<Vec<_>>();
    let entry = match matches.as_slice() {
        [] => {
            return Err(Diagnostic::error(format!(
                "server.client_package `{}` is not resolved in the active package configuration",
                config.package
            )));
        }
        [entry] => *entry,
        _ => {
            return Err(Diagnostic::error(format!(
                "server.client_package `{}` is ambiguous in the active package configuration",
                config.package
            )));
        }
    };
    let root_uri = entry
        .get("rootUri")
        .and_then(Value::as_str)
        .ok_or_else(|| Diagnostic::error("client package entry is missing rootUri"))?;
    let package_root =
        crate::source_uri::resolve_file_reference(&workspace.package_config.path, root_uri)
            .map_err(|error| {
                Diagnostic::error(format!("invalid client package rootUri: {error}"))
            })?;
    let package_root = validate_local_destination(workspace, &package_root, &config.package)?;
    Ok(ServerClientDestination {
        package_name: config.package.clone(),
        package_root,
        tags: config.tags.clone(),
    })
}

/// Ensures generated clients cannot escape to a cache or unrelated local checkout.
fn validate_local_destination(
    workspace: &WorkspacePlan,
    package_root: &Path,
    package_name: &str,
) -> Result<std::path::PathBuf, Diagnostic> {
    if !matches!(
        workspace.package_config.kind,
        PackageConfigKind::WorkspaceShared { .. }
    ) {
        return Err(Diagnostic::error(
            "server.client_package requires a local Dart pub workspace destination",
        ));
    }
    let workspace_root = workspace
        .package_config
        .path
        .parent()
        .and_then(|path| path.parent())
        .ok_or_else(|| Diagnostic::error("invalid shared package configuration path"))?;
    if !package_root.starts_with(workspace_root) {
        return Err(Diagnostic::error(format!(
            "server.client_package `{package_name}` resolves outside the active Dart workspace"
        )));
    }
    if package_root == workspace.package_root {
        return Err(Diagnostic::error(
            "server.client_package must name a separate client package",
        ));
    }
    let canonical_workspace = fs::canonicalize(workspace_root).map_err(|error| {
        Diagnostic::error(format!(
            "failed to resolve the active Dart workspace `{}`: {error}",
            workspace_root.display()
        ))
    })?;
    let canonical_package = fs::canonicalize(package_root).map_err(|error| {
        Diagnostic::error(format!(
            "failed to resolve server client package `{package_name}`: {error}"
        ))
    })?;
    if !canonical_package.starts_with(canonical_workspace) {
        return Err(Diagnostic::error(format!(
            "server.client_package `{package_name}` resolves outside the active Dart workspace"
        )));
    }
    if !canonical_package.join("lib").is_dir() {
        return Err(Diagnostic::error(format!(
            "server.client_package `{package_name}` has no `lib` directory"
        )));
    }
    let declared = load_package_name(&canonical_package)?;
    if declared != package_name {
        return Err(Diagnostic::error(format!(
            "server.client_package `{package_name}` resolves to package `{declared}`"
        )));
    }
    Ok(package_root.to_path_buf())
}
