use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use crate::source_uri::normalize_path;
use dust_dart_syntax::DartLanguageVersion;

/// SDK target include expansion and source identity alternatives.
mod map;

/// The active SDK's source map, retaining target-dependent declarations.
pub(super) struct SdkSources {
    /// SDK language version for sources without a package language version.
    pub(super) language_version: DartLanguageVersion,
    /// Metadata reads that affect source identity and parsing.
    pub(super) inputs: BTreeMap<PathBuf, String>,
    /// All distinct source paths for each SDK library, without choosing a host.
    libraries: BTreeMap<String, BTreeSet<PathBuf>>,
}

impl SdkSources {
    /// Explicit roots never fall back; discovery follows the first active Dart.
    pub(super) fn load(cwd: &Path, explicit: Option<&Path>) -> Result<Self, String> {
        let root = match explicit {
            Some(root) => normalize_path(&cwd.join(root)),
            None => {
                let process_cwd = std::env::current_dir()
                    .map_err(|error| format!("cannot discover Dart SDK sources: {error}"))?;
                discover(&process_cwd, std::env::var_os("PATH").as_deref())?
            }
        };
        Self::read(&root).map_err(|error| {
            format!(
                "cannot load Dart SDK sources at `{}`: {error}",
                root.display()
            )
        })
    }

    /// Reads metadata only; library source reads remain demand-driven.
    fn read(root: &Path) -> Result<Self, String> {
        let version_path = root.join("version");
        let version = read(&version_path)?;
        let language_version = sdk_language_version(&version)
            .ok_or_else(|| format!("invalid SDK version in `{}`", version_path.display()))?;
        let map::SourceMap {
            libraries,
            mut inputs,
            ..
        } = map::SourceMap::load(&root.join("lib/libraries.json"))?;
        if !libraries.contains_key("core") {
            return Err("SDK source map does not declare `dart:core`".into());
        }
        inputs.insert(version_path, version);
        Ok(Self {
            language_version,
            libraries,
            inputs,
        })
    }

    /// Rejects a relevant divergent target instead of guessing the host platform.
    pub(super) fn resolve(&self, name: &str) -> Result<PathBuf, String> {
        let paths = self
            .libraries
            .get(name)
            .ok_or_else(|| format!("SDK source map has no `dart:{name}` library"))?;
        if paths.len() != 1 {
            return Err(format!(
                "SDK library `dart:{name}` has target-dependent source identities"
            ));
        }
        Ok(paths.first().expect("source map entry is nonempty").clone())
    }
}

/// An SDK version file is a semantic version, not a free-form language constraint.
fn sdk_language_version(version: &str) -> Option<DartLanguageVersion> {
    let core = version.trim().split(['-', '+']).next()?;
    let components = core.split('.').collect::<Vec<_>>();
    if components.len() != 3
        || components.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return None;
    }
    components[2].parse::<u32>().ok()?;
    Some(DartLanguageVersion::new(
        components[0].parse().ok()?,
        components[1].parse().ok()?,
    ))
}

/// Finds installed sources beside a standalone binary or Flutter's Dart wrapper.
fn discover(cwd: &Path, path: Option<&OsStr>) -> Result<PathBuf, String> {
    for directory in path.into_iter().flat_map(std::env::split_paths) {
        let directory = cwd.join(directory);
        let names: &[&str] = if cfg!(windows) {
            &["dart.exe", "dart.bat", "dart.cmd"]
        } else {
            &["dart"]
        };
        for name in names {
            let executable = directory.join(name);
            if !is_executable(&executable) {
                continue;
            }
            let executable = fs::canonicalize(&executable)
                .map_err(|error| format!("cannot resolve `{}`: {error}", executable.display()))?;
            let bin = executable
                .parent()
                .ok_or("Dart executable has no directory")?;
            for root in [
                bin.parent().map(Path::to_path_buf),
                Some(bin.join("cache/dart-sdk")),
            ]
            .into_iter()
            .flatten()
            {
                if root.join("lib/libraries.json").is_file() {
                    return Ok(root);
                }
            }
            return Err(format!(
                "cannot discover SDK sources beside active Dart `{}`; supply sdk_source_root",
                executable.display()
            ));
        }
    }
    Err("cannot discover Dart SDK sources: no active Dart on PATH; supply sdk_source_root".into())
}

/// Non-executable files must not shadow the command selected by the shell.
fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Read failures retain the exact SDK input path.
fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read `{}`: {error}", path.display()))
}

#[cfg(test)]
mod tests;
