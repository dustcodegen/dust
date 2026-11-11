use std::{
    cmp::Reverse,
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
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
    /// Package roots ordered deepest first for file language-version selection.
    file_roots: Vec<(PathBuf, Option<DartLanguageVersion>)>,
    /// Active SDK libraries and their target-independent source alternatives.
    sdk: SdkSources,
    /// Exact package and SDK metadata reads for dependency fingerprints.
    pub(super) inputs: BTreeMap<PathBuf, String>,
    /// Base-independent package and SDK directive resolutions for this snapshot.
    absolute: Mutex<BTreeMap<String, Result<Source, String>>>,
}

impl SourceIndex {
    /// Reads package configuration and the selected SDK without resolving by guesses.
    pub(super) fn load(
        cwd: &Path,
        config: &Path,
        sdk_root: Option<&Path>,
        is_flutter_workspace: bool,
    ) -> Result<Self, String> {
        let contents = fs::read_to_string(config).map_err(|error| {
            format!(
                "cannot read package configuration `{}`: {error}",
                config.display()
            )
        })?;
        let packages = packages::load(config, &contents)?;
        let flutter_package = if sdk_root.is_none() && is_flutter_workspace {
            Some(
                &packages
                    .get("flutter")
                    .ok_or("Flutter workspace package configuration does not resolve `flutter`")?
                    .root,
            )
        } else {
            None
        };
        let sdk = SdkSources::load(cwd, sdk_root, flutter_package.map(PathBuf::as_path))?;
        let mut inputs = sdk.inputs.clone();
        inputs.insert(config.to_path_buf(), contents);
        let mut file_roots = packages
            .values()
            .rev()
            .map(|package| (package.root.clone(), package.language_version))
            .collect::<Vec<_>>();
        // A stable sort retains the last package-name preference for equal depths.
        file_roots.sort_by_key(|(root, _)| Reverse(root.components().count()));
        Ok(Self {
            packages,
            file_roots,
            sdk,
            inputs,
            absolute: Mutex::new(BTreeMap::new()),
        })
    }

    /// File entrypoints retain file identity, even when a package URI maps there.
    pub(super) fn file(&self, path: &Path) -> Result<Source, String> {
        let path = normalize_path(path);
        let language_version = self
            .file_roots
            .iter()
            .find(|(root, _)| path.starts_with(root))
            .and_then(|(_, language_version)| *language_version)
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
            let package = scheme.eq_ignore_ascii_case("package");
            if package || scheme.eq_ignore_ascii_case("dart") {
                if let Some(resolved) = self
                    .absolute
                    .lock()
                    .expect("source index cache lock")
                    .get(reference)
                    .cloned()
                {
                    return resolved;
                }
                let resolved = if package {
                    self.package(path)
                } else {
                    self.dart(path)
                };
                self.absolute
                    .lock()
                    .expect("source index cache lock")
                    .insert(reference.to_owned(), resolved.clone());
                return resolved;
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
