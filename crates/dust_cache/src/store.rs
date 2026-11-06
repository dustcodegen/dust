use std::{
    collections::BTreeMap,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use dust_plugin_api::LibraryAnalysisSnapshot;
use serde::{Deserialize, Serialize};

/// The current on-disk Dust cache schema version.
pub const CACHE_SCHEMA_VERSION: u32 = 7;

/// One cached build fingerprint for a source library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The current source file fingerprint.
    pub source_hash: u64,
    /// The current workspace configuration fingerprint.
    pub package_config_hash: u64,
    /// The current Dust code generation fingerprint.
    #[serde(default)]
    pub tool_hash: u64,
    /// The expected generated output fingerprint for the current source state.
    pub expected_output_hash: u64,
    /// Additional generated output paths owned by this library build.
    #[serde(default)]
    pub auxiliary_output_paths: Vec<PathBuf>,
    /// Whether this library intentionally leaves the primary output absent.
    #[serde(default)]
    pub suppress_primary_output: bool,
    /// Hash of the workspace analysis that was active when this entry was built.
    #[serde(default)]
    pub workspace_analysis_hash: u64,
    /// The cached parse-only plugin analysis facts for this library.
    #[serde(default)]
    pub analysis_snapshot: LibraryAnalysisSnapshot,
}

/// Owned cache file shape read from disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CacheFile {
    /// Cache schema version stored on disk.
    schema_version: u32,
    /// Cached entries keyed by workspace-relative source path.
    entries: BTreeMap<String, CacheEntry>,
}

/// Borrowed cache file view used for serialization.
#[derive(Debug, Serialize)]
struct CacheFileRef<'a> {
    /// Cache schema version to write.
    schema_version: u32,
    /// Cached entries keyed by workspace-relative source path.
    entries: &'a BTreeMap<String, CacheEntry>,
}

/// The persistent cache for one Dart workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCache {
    /// Path to the cache JSON file on disk.
    path: PathBuf,
    /// Cached entries keyed by workspace-relative source path.
    entries: BTreeMap<String, CacheEntry>,
    /// Whether the in-memory cache has changes that need flushing.
    dirty: bool,
}

impl WorkspaceCache {
    /// Loads the workspace cache from `.dart_tool/dust/build_cache_v1.json`.
    ///
    /// Missing cache files are treated as an empty cache.
    pub fn load(root: &Path) -> io::Result<Self> {
        let path = cache_file_path(root);
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Self::empty(path));
            }
            Err(error) => return Err(error),
        };

        let parsed = serde_json::from_str::<CacheFile>(&contents)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if parsed.schema_version != CACHE_SCHEMA_VERSION {
            return Ok(Self::empty(path));
        }

        Ok(Self {
            path,
            entries: parsed.entries,
            dirty: false,
        })
    }

    /// Opens the cache, treating malformed build metadata as a miss.
    ///
    /// The returned error describes any malformed contents that were ignored.
    /// Other read errors are returned normally.
    pub fn load_with_recovery(root: &Path) -> io::Result<(Self, Option<io::Error>)> {
        match Self::load(root) {
            Ok(cache) => Ok((cache, None)),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                Ok((Self::empty(cache_file_path(root)), Some(error)))
            }
            Err(error) => Err(error),
        }
    }

    /// Constructs a cache with no saved entries.
    fn empty(path: PathBuf) -> Self {
        Self {
            path,
            entries: BTreeMap::new(),
            dirty: false,
        }
    }

    /// Returns the on-disk cache file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the cache entry for one source file if present.
    pub fn get(&self, root: &Path, source_path: &Path) -> Option<&CacheEntry> {
        self.entries.get(&cache_key(root, source_path))
    }

    /// Inserts or replaces the cache entry for one source file.
    pub fn insert(&mut self, root: &Path, source_path: &Path, entry: CacheEntry) {
        let key = cache_key(root, source_path);
        if self.entries.get(&key) != Some(&entry) {
            self.dirty = true;
        }
        self.entries.insert(key, entry);
    }

    /// Removes the cache entry for one source file.
    pub fn remove(&mut self, root: &Path, source_path: &Path) {
        if self.entries.remove(&cache_key(root, source_path)).is_some() {
            self.dirty = true;
        }
    }

    /// Persists the current cache contents to disk.
    pub fn flush(&mut self) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }

        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;

        let file = CacheFileRef {
            schema_version: CACHE_SCHEMA_VERSION,
            entries: &self.entries,
        };
        let json = serde_json::to_vec(&file)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        #[cfg(unix)]
        let existing_permissions = match fs::metadata(&self.path) {
            Ok(metadata) => Some(metadata.permissions()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        #[cfg(unix)]
        let mut temporary = tempfile::Builder::new()
            .permissions(
                existing_permissions
                    .clone()
                    .unwrap_or_else(|| fs::Permissions::from_mode(0o666)),
            )
            .tempfile_in(parent)?;
        #[cfg(not(unix))]
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(&json)?;
        // The umask only applies to creating a new cache. Keep an existing cache's mode.
        #[cfg(unix)]
        if let Some(permissions) = existing_permissions {
            temporary.as_file().set_permissions(permissions)?;
        }
        temporary.persist(&self.path).map_err(|error| error.error)?;
        self.dirty = false;
        Ok(())
    }

    /// Deletes the entire Dust cache storage directory under `.dart_tool`.
    ///
    /// Returns `Ok(true)` when the directory existed and was removed.
    /// Returns `Ok(false)` when no Dust cache storage was present.
    pub fn delete_storage(root: &Path) -> io::Result<bool> {
        let path = cache_dir_path(root);
        match fs::remove_dir_all(&path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }
}

/// Builds the normalized cache key for a source path.
fn cache_key(root: &Path, source_path: &Path) -> String {
    source_path
        .strip_prefix(root)
        .unwrap_or(source_path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Returns the workspace cache file path.
fn cache_file_path(root: &Path) -> PathBuf {
    cache_dir_path(root).join("build_cache_v1.json")
}

/// Returns the Dust cache storage directory for a workspace.
fn cache_dir_path(root: &Path) -> PathBuf {
    root.join(".dart_tool/dust")
}

#[cfg(test)]
#[path = "store/tests.rs"]
mod tests;
