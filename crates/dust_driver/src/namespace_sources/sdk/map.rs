use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::source_uri::resolve_file_reference;

/// A target belongs to one specification, including locally included maps.
type TargetKey = (PathBuf, String);
/// Effective library paths after ordered include and local overrides.
type Libraries = BTreeMap<String, PathBuf>;

/// Local SDK specification reads and resolved targets, including transitive includes.
#[derive(Default)]
pub(super) struct SourceMap {
    /// Exact metadata text for later read-set fingerprints.
    pub(super) inputs: BTreeMap<PathBuf, String>,
    /// Source alternatives retained independently of the host target.
    pub(super) libraries: BTreeMap<String, BTreeSet<PathBuf>>,
    /// Parsed local specifications shared across target traversals.
    maps: BTreeMap<PathBuf, Value>,
    /// Physical identities prevent include cycles disguised by symlink aliases.
    identities: BTreeMap<PathBuf, PathBuf>,
    /// Completed target include expansion, preserving override order.
    targets: BTreeMap<TargetKey, Libraries>,
    /// Current recursion stack; missing targets and cycles are not empty maps.
    active: BTreeSet<TargetKey>,
}

impl SourceMap {
    /// Union the root specification's targets only, after applying each include.
    pub(super) fn load(path: &Path) -> Result<Self, String> {
        let mut map = Self::default();
        let targets = map.specification(path)?;
        for target in targets
            .as_object()
            .expect("validated SDK source map")
            .keys()
            .filter(|target| !target.starts_with("comment:"))
        {
            for (name, source) in map.target(path, target)? {
                map.libraries.entry(name).or_default().insert(source);
            }
        }
        Ok(map)
    }

    /// Retain exact read/parse errors rather than treating a map as absent.
    fn specification(&mut self, path: &Path) -> Result<Value, String> {
        if let Some(value) = self.maps.get(path) {
            return Ok(value.clone());
        }
        let contents = super::read(path)?;
        let value: Value = serde_json::from_str(&contents)
            .map_err(|error| format!("invalid `{}`: {error}", path.display()))?;
        if !value.is_object() {
            return Err("SDK source map must be an object".into());
        }
        self.inputs.insert(path.to_path_buf(), contents);
        self.identities
            .insert(path.to_path_buf(), physical_path(path)?);
        self.maps.insert(path.to_path_buf(), value.clone());
        Ok(value)
    }

    /// Later includes override earlier ones, then the target's own entries win.
    fn target(&mut self, path: &Path, name: &str) -> Result<Libraries, String> {
        let key = (path.to_path_buf(), name.to_owned());
        if let Some(libraries) = self.targets.get(&key) {
            return Ok(libraries.clone());
        }
        if !self.active.insert(key.clone()) {
            return Err(format!(
                "cyclic SDK target include `{name}` in `{}`",
                path.display()
            ));
        }
        let specification = self.specification(path)?;
        let target = specification
            .get(name)
            .and_then(Value::as_object)
            .ok_or_else(|| {
                format!(
                    "missing or invalid SDK target `{name}` in `{}`",
                    path.display()
                )
            })?;
        let mut libraries = BTreeMap::new();
        if let Some(include) = target.get("include").filter(|value| !value.is_null()) {
            for include in include
                .as_array()
                .ok_or_else(|| format!("SDK target `{name}` include must be a list"))?
            {
                let target = include
                    .get("target")
                    .and_then(Value::as_str)
                    .ok_or("SDK include must name a target")?;
                let included_path = match include.get("path") {
                    Some(value) => {
                        let uri = value
                            .as_str()
                            .ok_or("SDK include path must be a URI string")?;
                        let included = resolve_file_reference(path, uri)?;
                        let physical = physical_path(&included)?;
                        if self
                            .active
                            .iter()
                            .any(|(active, _)| self.identities.get(active) == Some(&physical))
                        {
                            return Err(format!(
                                "cyclic SDK source-map include `{}`",
                                included.display()
                            ));
                        }
                        included
                    }
                    None => path.to_path_buf(),
                };
                libraries.extend(self.target(&included_path, target)?);
            }
        }
        let entries = target
            .get("libraries")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("SDK target `{name}` has no library map"))?;
        for (name, entry) in entries {
            let uri = entry
                .get("uri")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("SDK library `{name}` has no source URI"))?;
            libraries.insert(name.clone(), resolve_file_reference(path, uri)?);
        }
        self.active.remove(&key);
        self.targets.insert(key, libraries.clone());
        Ok(libraries)
    }
}

/// Canonical identities are used for cycle checks, not relative URI resolution.
fn physical_path(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path)
        .map_err(|error| format!("cannot read SDK map `{}`: {error}", path.display()))
}
