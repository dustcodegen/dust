use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use dust_dart_syntax::DartLanguageVersion;

use crate::source_uri::{normalize_path, resolve_file_reference};

use super::{sdk::SdkSources, uri};

/// Strict package-config metadata and package/language-version validation.
mod packages;

/// Canonical import identity and actual local source location remain separate.
#[derive(Debug, Clone)]
pub(super) struct Source {
    /// Canonical library identity, independent of its physical source path.
    pub(super) uri: String,
    /// Local file to read without fetching network imports.
    pub(super) path: PathBuf,
    /// Effective package or SDK language version for parsing this source.
    pub(super) language_version: DartLanguageVersion,
}

/// Actual package configuration and SDK source map used by one graph snapshot.
pub(super) struct SourceIndex {
    /// Resolved package entries keyed by their configured names.
    packages: BTreeMap<String, packages::Package>,
    /// Active SDK libraries and their target-independent source alternatives.
    sdk: SdkSources,
    /// Exact package and SDK metadata reads for dependency fingerprints.
    pub(super) inputs: BTreeMap<PathBuf, String>,
}

impl SourceIndex {
    /// Reads package configuration and the selected SDK without resolving by guesses.
    pub(super) fn load(cwd: &Path, config: &Path, sdk_root: Option<&Path>) -> Result<Self, String> {
        let contents = fs::read_to_string(config).map_err(|error| {
            format!(
                "cannot read package configuration `{}`: {error}",
                config.display()
            )
        })?;
        let packages = packages::load(config, &contents)?;
        let sdk = SdkSources::load(cwd, sdk_root)?;
        let mut inputs = sdk.inputs.clone();
        inputs.insert(config.to_path_buf(), contents);
        Ok(Self {
            packages,
            sdk,
            inputs,
        })
    }

    /// File entrypoints retain file identity, even when a package URI maps there.
    pub(super) fn file(&self, path: &Path) -> Result<Source, String> {
        let path = normalize_path(path);
        let language_version = self
            .packages
            .values()
            .filter(|package| path.starts_with(&package.root))
            .max_by_key(|package| package.root.components().count())
            .and_then(|package| package.language_version)
            .unwrap_or(self.sdk.language_version);
        Ok(Source {
            uri: uri::file_uri(&path)?,
            path,
            language_version,
        })
    }

    /// Resolves a structured directive URI while retaining its library identity.
    pub(super) fn resolve(&self, base: &Source, reference: &str) -> Result<Source, String> {
        if reference.contains(['?', '#']) {
            return Err("source URI must not contain a query or fragment".into());
        }
        if let Some((scheme, path)) = reference.split_once(':').filter(|(s, _)| !s.contains('/')) {
            if scheme.eq_ignore_ascii_case("package") {
                return self.package(path);
            }
            if scheme.eq_ignore_ascii_case("dart") {
                return self.dart(path);
            }
            let path = resolve_file_reference(&base.path, reference)?;
            let mut source = self.file(&path)?;
            source.uri = uri::file_reference(&uri::file_uri(&base.path)?, reference)?;
            return Ok(source);
        }
        if reference.starts_with("//") {
            return Err("source URI must not introduce an authority".into());
        }
        if let Some(path) = base.uri.strip_prefix("package:") {
            let (name, path) = path.split_once('/').ok_or("invalid package URI base")?;
            return self.package(&format!("{name}/{}", uri::relative_path(path, reference)?));
        }
        if let Some(path) = base.uri.strip_prefix("dart:") {
            let name = path.split('/').next().ok_or("invalid SDK URI base")?;
            let root = self.sdk.resolve(name)?;
            let path = resolve_file_reference(&base.path, reference)?;
            if path == root {
                return self.dart(name);
            }
            let relative = path
                .strip_prefix(root.parent().ok_or("SDK source has no directory")?)
                .map_err(|_| "relative SDK URI escapes its declaring library directory")?;
            return self.dart(&format!(
                "{name}/{}",
                relative.to_str().ok_or("non-UTF-8 SDK source URI")?
            ));
        }
        let path = resolve_file_reference(&base.path, reference)?;
        let mut source = self.file(&path)?;
        source.uri = uri::file_reference(&base.uri, reference)?;
        Ok(source)
    }

    /// Resolves package name/path against the actual package-config entry.
    fn package(&self, path: &str) -> Result<Source, String> {
        let path = uri::encode_path(path)?;
        let (name, relative) = path
            .split_once('/')
            .ok_or("package URI must contain name/path")?;
        if !packages::valid_name(name) {
            return Err(format!("invalid package URI name `{name}`"));
        }
        let package = self
            .packages
            .get(name)
            .ok_or_else(|| format!("package configuration does not resolve `{name}`"))?;
        let normalized = uri::normalize(&format!("/{relative}"));
        if relative
            .split('/')
            .try_fold(0_usize, |depth, part| match part {
                ".." => depth.checked_sub(1),
                "." => Some(depth),
                _ => Some(depth + 1),
            })
            .is_none()
        {
            return Err("package URI escapes its package name".into());
        }
        let path = if relative.is_empty() {
            package.library_root.clone()
        } else {
            resolve_file_reference(&package.library_root.join(".uri-base"), relative)?
        };
        Ok(Source {
            uri: format!("package:{name}{normalized}"),
            path,
            language_version: package
                .language_version
                .unwrap_or(self.sdk.language_version),
        })
    }

    /// Resolves an SDK library or part using its declared source-map identity.
    fn dart(&self, name: &str) -> Result<Source, String> {
        let (name, relative) = name
            .split_once('/')
            .map_or((name, None), |(name, rest)| (name, Some(rest)));
        let root = self.sdk.resolve(name)?;
        let path = match relative {
            None => root,
            Some(relative) => resolve_file_reference(&root, relative)?,
        };
        Ok(Source {
            uri: format!(
                "dart:{name}{}",
                relative.map_or(String::new(), |value| format!("/{}", uri::normalize(value)))
            ),
            path,
            language_version: self.sdk.language_version,
        })
    }
}

#[cfg(test)]
mod tests;
