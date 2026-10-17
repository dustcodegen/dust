use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use dust_diagnostics::Diagnostic;
use serde_json::Value;

use super::{is_dust_package, version::Version};

/// Resolved package roots keyed by Dust package name.
pub(super) fn resolved_dust_package_roots(
    package_config_path: &Path,
) -> Result<BTreeMap<String, PathBuf>, Diagnostic> {
    let contents = fs::read_to_string(package_config_path).map_err(|error| {
        Diagnostic::error(format!(
            "failed to read package configuration `{}`: {error}",
            package_config_path.display()
        ))
    })?;
    let value = serde_json::from_str::<Value>(&contents).map_err(|error| {
        Diagnostic::error(format!(
            "failed to parse package configuration `{}`: {error}",
            package_config_path.display()
        ))
    })?;
    let Some(packages) = value.get("packages").and_then(Value::as_array) else {
        return Ok(BTreeMap::new());
    };

    let mut roots = BTreeMap::new();
    for package in packages {
        let Some(name) = package.get("name").and_then(Value::as_str) else {
            continue;
        };
        if !is_dust_package(name) {
            continue;
        }
        let root_uri = package
            .get("rootUri")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                Diagnostic::error(format!(
                    "package configuration entry for `{name}` is missing `rootUri`"
                ))
            })?;
        roots.insert(
            name.to_owned(),
            resolve_package_root_uri(package_config_path, root_uri)?,
        );
    }

    Ok(roots)
}

/// Resolves a `package_config.json` root URI to a filesystem path.
fn resolve_package_root_uri(
    package_config_path: &Path,
    root_uri: &str,
) -> Result<PathBuf, Diagnostic> {
    crate::source_uri::resolve_file_reference(package_config_path, root_uri).map_err(|error| {
        Diagnostic::error(format!("invalid package root URI `{root_uri}`: {error}"))
    })
}

/// Reads and parses one resolved Dust package pubspec version.
pub(super) fn read_package_version(
    package_root: &Path,
    package: &str,
) -> Result<(String, Version), Diagnostic> {
    let pubspec_path = package_root.join("pubspec.yaml");
    let contents = fs::read_to_string(&pubspec_path).map_err(|error| {
        Diagnostic::error(format!(
            "failed to read resolved `{package}` pubspec `{}`: {error}",
            pubspec_path.display()
        ))
    })?;
    let version_text = contents
        .lines()
        .find_map(pubspec_version_value)
        .ok_or_else(|| {
            Diagnostic::error(format!(
                "resolved `{package}` pubspec `{}` does not declare `version`",
                pubspec_path.display()
            ))
        })?;
    let version = Version::parse(&version_text).ok_or_else(|| {
        Diagnostic::error(format!(
            "resolved `{package}` has unsupported version `{version_text}`"
        ))
    })?;

    Ok((version_text, version))
}

/// Extracts a simple top-level `version:` value from a pubspec line.
fn pubspec_version_value(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let value = trimmed.strip_prefix("version:")?.split('#').next()?.trim();
    Some(value.trim_matches('"').trim_matches('\'').trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::resolve_package_root_uri;

    #[test]
    fn file_uri_resolution_keeps_localhost_paths_absolute() {
        let path = resolve_package_root_uri(
            Path::new("/workspace/.dart_tool/package_config.json"),
            "file://localhost/tmp/dust%20pkg",
        )
        .unwrap();

        assert_eq!(path, PathBuf::from("/tmp/dust pkg"));
    }

    #[test]
    fn package_roots_resolve_uri_segments_before_becoming_paths() {
        let base = Path::new("/workspace/.dart_tool/package_config.json");
        for (uri, expected) in [
            ("../deps/./pkg/../real%20pkg", "/workspace/deps/real pkg"),
            ("../deps/%2e%2e/real", "/workspace/real"),
            ("file:///tmp/%E2%98%83%23name", "/tmp/☃#name"),
            ("file:relative", "/relative"),
        ] {
            assert_eq!(
                resolve_package_root_uri(base, uri).unwrap(),
                PathBuf::from(expected),
                "{uri}"
            );
        }
    }

    #[test]
    fn non_file_or_unrepresentable_roots_are_not_local_paths() {
        let base = Path::new("/workspace/.dart_tool/package_config.json");
        for uri in [
            "https://example.invalid/pkg",
            "package:foreign/root",
            "file://user@remote.invalid/root",
            "../root?query",
            "../root#fragment",
            "../a%2Fb",
            "../%00bad",
            "../%FF",
            "../%2",
            "../%GG",
        ] {
            assert!(resolve_package_root_uri(base, uri).is_err(), "{uri}");
        }
    }
}
