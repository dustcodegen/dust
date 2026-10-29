use std::{collections::BTreeMap, path::Path};

use dust_diagnostics::Diagnostic;
use serde::Deserialize;

use super::invalid_config;

/// Generated-server settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    /// Local Dart packages that receive backend-derived clients.
    pub clients: Vec<ServerClientConfig>,
}

/// One generated-client destination and its optional route-tag filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerClientConfig {
    /// Exact destination package name from `pubspec.yaml`.
    pub package: String,
    /// Route tags accepted by this destination; empty means every route.
    pub tags: Vec<String>,
}

/// Deserialized `server` section from `dust.yaml`.
#[derive(Debug, Default, Deserialize)]
pub(super) struct RawServerConfig {
    /// One package that receives every generated client.
    client_package: Option<String>,
    /// Destination package names mapped to accepted route tags.
    client_packages: Option<BTreeMap<String, Vec<String>>>,
}

/// Parses optional generated-server destinations.
pub(super) fn parse_server_config(
    raw: Option<RawServerConfig>,
    path: &Path,
) -> Result<Option<ServerConfig>, Diagnostic> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let (clients, require_tags) = match (raw.client_package, raw.client_packages) {
        (Some(package), None) => (
            vec![ServerClientConfig {
                package,
                tags: Vec::new(),
            }],
            false,
        ),
        (None, Some(packages)) if !packages.is_empty() => (
            packages
                .into_iter()
                .map(|(package, tags)| ServerClientConfig { package, tags })
                .collect(),
            true,
        ),
        (Some(_), Some(_)) => {
            return Err(invalid_config(
                path,
                "server.client_package and server.client_packages are mutually exclusive",
            ));
        }
        _ => {
            return Err(invalid_config(
                path,
                "server.client_package or server.client_packages is required",
            ));
        }
    };
    validate_clients(&clients, require_tags, path)?;
    Ok(Some(ServerConfig { clients }))
}

/// Validates package identities and split-mode route tags.
fn validate_clients(
    clients: &[ServerClientConfig],
    require_tags: bool,
    path: &Path,
) -> Result<(), Diagnostic> {
    for client in clients {
        if !valid_package_name(&client.package) {
            return Err(invalid_config(
                path,
                "server.client_package and server.client_packages keys must be valid Dart package names",
            ));
        }
        if require_tags && (client.tags.is_empty() || client.tags.iter().any(String::is_empty)) {
            return Err(invalid_config(
                path,
                "server.client_packages tag lists must contain non-empty tags",
            ));
        }
    }
    Ok(())
}

/// Accepts Dart package names without treating a path as package identity.
fn valid_package_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
