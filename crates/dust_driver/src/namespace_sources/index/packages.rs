use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use dust_dart_syntax::DartLanguageVersion;
use serde_json::Value;

use crate::source_uri::resolve_file_reference;

/// Resolved package-config paths, not ownership guesses based on package spelling.
pub(super) struct Package {
    /// Root directory resolved relative to the package-config location.
    pub(super) root: PathBuf,
    /// Directory used to resolve the package's public library URIs.
    pub(super) library_root: PathBuf,
    /// Explicit language override, or the active SDK language when absent.
    pub(super) language_version: Option<DartLanguageVersion>,
}

/// Validates package-config v2 entries and resolves their local URI paths.
pub(super) fn load(path: &Path, contents: &str) -> Result<BTreeMap<String, Package>, String> {
    let value: Value = serde_json::from_str(contents).map_err(|error| {
        format!(
            "invalid package configuration `{}`: {error}",
            path.display()
        )
    })?;
    if value.get("configVersion").and_then(Value::as_u64) != Some(2) {
        return Err("package configuration must declare configVersion 2".into());
    }
    let entries = value
        .get("packages")
        .and_then(Value::as_array)
        .ok_or("package configuration must declare a packages list")?;
    let mut packages = BTreeMap::new();
    for entry in entries {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .ok_or("package is missing name")?;
        if !valid_name(name) {
            return Err(format!("invalid package name `{name}`"));
        }
        let root_uri = entry
            .get("rootUri")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("package `{name}` is missing rootUri"))?;
        let root = resolve_file_reference(path, root_uri)?;
        let library_root = match entry.get("packageUri") {
            None => root.clone(),
            Some(value) => {
                let uri = value
                    .as_str()
                    .ok_or_else(|| format!("invalid packageUri for `{name}`"))?;
                if uri.is_empty() {
                    root.clone()
                } else {
                    resolve_file_reference(&root.join(".uri-base"), uri)?
                }
            }
        };
        if !library_root.starts_with(&root) {
            return Err(format!(
                "package `{name}` packageUri is outside its rootUri"
            ));
        }
        let language_version = match entry.get("languageVersion") {
            None => None,
            Some(value) => {
                Some(value.as_str().and_then(language_version).ok_or_else(|| {
                    format!("invalid or unsupported languageVersion for `{name}`")
                })?)
            }
        };
        if packages
            .values()
            .any(|package: &Package| package.root == root)
        {
            return Err(format!("duplicate package root `{}`", root.display()));
        }
        if packages
            .insert(
                name.to_owned(),
                Package {
                    root,
                    library_root,
                    language_version,
                },
            )
            .is_some()
        {
            return Err(format!("duplicate package name `{name}`"));
        }
    }
    Ok(packages)
}

/// Accepts package-name URI segments without escaped separators or dot-only names.
pub(super) fn valid_name(name: &str) -> bool {
    name.bytes().any(|byte| byte != b'.')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!$&'()*+,-.;=@_~".contains(&byte))
}

/// Parses the package-config major.minor form without accepting SDK constraints.
fn language_version(value: &str) -> Option<DartLanguageVersion> {
    let (major, minor) = value.split_once('.')?;
    for part in [major, minor] {
        if part.is_empty()
            || !part.bytes().all(|byte| byte.is_ascii_digit())
            || (part.len() > 1 && part.starts_with('0'))
        {
            return None;
        }
    }
    Some(DartLanguageVersion::new(
        major.parse().ok()?,
        minor.parse().ok()?,
    ))
}
