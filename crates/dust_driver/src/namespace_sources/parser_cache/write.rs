use std::{
    collections::HashSet,
    fs,
    io::{self, Write},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use super::*;

/// Caller joins this optional write before recording elapsed command time.
pub(crate) fn save(
    namespaces: &NamespaceSources,
    root: &Path,
    annotations: &[&str],
) -> io::Result<()> {
    let roots = namespaces
        .roots
        .values()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let eligible = || {
        namespaces.units.iter().filter_map(|(uri, unit)| {
            let unit = unit.as_ref().ok()?;
            (unit.generated_owner.is_none()
                && !unit.generated_library
                && unit.diagnostics.is_empty()
                && unit.namespace_diagnostics.is_empty())
            .then_some((uri.as_str(), unit))
        })
    };
    let header = format::Header {
        identity: crate::build::parser_identity(),
        annotations: annotations.iter().map(|name| (*name).to_owned()).collect(),
    };
    let mut bytes = format::begin(&header, eligible().count()).map_err(io::Error::other)?;
    for (uri, unit) in eligible() {
        // Only one temporary AST blob is retained alongside the final snapshot buffer.
        let library = postcard::to_stdvec(&unit.parsed).map_err(io::Error::other)?;
        format::append_record(
            &mut bytes,
            uri,
            format::options(ParseOptions {
                language_version: unit.source.language_version,
                ..Default::default()
            }),
            roots.contains(uri),
            unit.text.as_bytes(),
            &library,
        );
    }
    format::seal(&mut bytes);
    persist(&path(root), &bytes)
}

/// Atomic publication retains the old cache on failure and never writes through a symlink.
fn persist(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    #[cfg(unix)]
    let existing_permissions = fs::symlink_metadata(path)
        .ok()
        .filter(|metadata| metadata.file_type().is_file())
        .map(|metadata| metadata.permissions());
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
    temporary.write_all(bytes)?;
    #[cfg(unix)]
    if let Some(permissions) = existing_permissions {
        temporary.as_file().set_permissions(permissions)?;
    }
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}
