use std::{fs, path::PathBuf};

use dust_diagnostics::Diagnostic;
use dust_server_plugin::ServerClientDestination;
use dust_workspace::{PackageConfigKind, WorkspacePlan, load_package_name};
use serde_json::Value;

#[cfg(test)]
mod tests;

/// Resolves the configured generated-client destination inside the active pub workspace.
pub(crate) fn resolve_destination(
    workspace: &WorkspacePlan,
) -> Result<Option<ServerClientDestination>, Diagnostic> {
    let Some(config) = &workspace.dust_config.server else {
        return Ok(None);
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
    let matches = entries
        .iter()
        .filter(|entry| entry.get("name").and_then(Value::as_str) == Some(&config.client_package))
        .collect::<Vec<_>>();
    let entry = match matches.as_slice() {
        [] => {
            return Err(Diagnostic::error(format!(
                "server.client_package `{}` is not resolved in the active package configuration",
                config.client_package
            )));
        }
        [entry] => *entry,
        _ => {
            return Err(Diagnostic::error(format!(
                "server.client_package `{}` is ambiguous in the active package configuration",
                config.client_package
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
    validate_local_destination(workspace, &package_root, &config.client_package)?;
    Ok(Some(ServerClientDestination {
        package_name: config.client_package.clone(),
        package_root,
    }))
}

/// Ensures generated clients cannot escape to a cache or unrelated local checkout.
fn validate_local_destination(
    workspace: &WorkspacePlan,
    package_root: &PathBuf,
    package_name: &str,
) -> Result<(), Diagnostic> {
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
    if package_root == &workspace.package_root {
        return Err(Diagnostic::error(
            "server.client_package must name a separate client package",
        ));
    }
    if !package_root.join("lib").is_dir() {
        return Err(Diagnostic::error(format!(
            "server.client_package `{package_name}` has no `lib` directory"
        )));
    }
    let declared = load_package_name(package_root)?;
    if declared != package_name {
        return Err(Diagnostic::error(format!(
            "server.client_package `{package_name}` resolves to package `{declared}`"
        )));
    }
    Ok(())
}
