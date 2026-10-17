use std::path::{Component, Path, PathBuf};

/// Resolves a local source URI reference without interpreting other schemes as paths.
pub(crate) fn resolve_file_reference(base_file: &Path, reference: &str) -> Result<PathBuf, String> {
    if reference.contains(['?', '#']) {
        return Err("local source URI must not contain a query or fragment".into());
    }
    let scheme = reference
        .split_once(':')
        .filter(|(scheme, _)| !scheme.contains('/'));
    let path = match scheme {
        Some((scheme, path)) if scheme.eq_ignore_ascii_case("file") => file_path(path)?,
        Some((scheme, _)) => return Err(format!("unsupported local source URI scheme `{scheme}`")),
        None if reference.starts_with("//") => file_path(reference)?,
        None if reference.starts_with('/') => file_path(reference)?,
        None if reference.is_empty() => base_file.to_path_buf(),
        None => base_file
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(decode_path(reference)?),
    };
    Ok(normalize_path(&path))
}

/// Converts file URI authority/path facts, never fetching a remote URI.
fn file_path(value: &str) -> Result<PathBuf, String> {
    let (authority, path) = match value.strip_prefix("//") {
        Some(rest) => rest.split_once('/').map_or((rest, ""), |(authority, _)| {
            (authority, &rest[authority.len()..])
        }),
        None => ("", value),
    };
    let path = decode_path(path)?;
    if !authority.is_empty() && !authority.eq_ignore_ascii_case("localhost") {
        if !cfg!(windows) || authority.contains(['@', ':', '%', '\\', '[', ']']) {
            return Err(format!("unsupported file URI authority `{authority}`"));
        }
        return Ok(PathBuf::from(format!(
            "//{authority}/{}",
            path.trim_start_matches('/')
        )));
    }
    let path = format!("/{}", path.trim_start_matches('/'));
    #[cfg(windows)]
    let path = if path.as_bytes().get(2) == Some(&b':') {
        if !path.as_bytes()[1].is_ascii_alphabetic() || path.as_bytes().get(3) != Some(&b'/') {
            return Err("invalid file URI drive path".into());
        }
        &path[1..]
    } else {
        &path
    };
    Ok(PathBuf::from(path))
}

/// URI segments are decoded separately so an escaped slash cannot change identity.
fn decode_path(value: &str) -> Result<String, String> {
    value
        .split('/')
        .map(|segment| {
            let mut decoded = Vec::with_capacity(segment.len());
            let mut bytes = segment.bytes();
            while let Some(byte) = bytes.next() {
                let byte = if byte == b'%' {
                    let hi = bytes.next().and_then(hex);
                    let lo = bytes.next().and_then(hex);
                    hi.zip(lo)
                        .map(|(hi, lo)| hi << 4 | lo)
                        .ok_or("invalid percent escape in source URI")?
                } else {
                    byte
                };
                if byte == 0 || byte == b'/' || (cfg!(windows) && byte == b'\\') {
                    return Err("invalid character in source URI path segment".into());
                }
                decoded.push(byte);
            }
            String::from_utf8(decoded).map_err(|_| "source URI path is not valid UTF-8".into())
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|segments| segments.join("/"))
}

/// Decodes one ASCII hexadecimal digit.
fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Removes URI dot segments without requiring the final source file to exist.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir if result.file_name().is_some_and(|name| name != "..") => {
                result.pop();
            }
            Component::ParentDir if result.has_root() => {}
            other => result.push(other.as_os_str()),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_parent_segments_do_not_cancel_each_other() {
        for (source, expected) in [
            ("../../file.dart", "../../file.dart"),
            ("one/../../file.dart", "../file.dart"),
            ("/../../file.dart", "/file.dart"),
            ("./one/../two.dart", "two.dart"),
        ] {
            assert_eq!(normalize_path(Path::new(source)), PathBuf::from(expected));
        }
    }

    #[test]
    fn empty_absolute_and_case_insensitive_file_references_have_stable_paths() {
        let base = Path::new("/workspace/.dart_tool/package_config.json");
        for (reference, expected) in [
            ("", "/workspace/.dart_tool/package_config.json"),
            ("/absolute/../source.dart", "/source.dart"),
            ("FILE:///tmp/a%25b.dart", "/tmp/a%b.dart"),
            ("//localhost/tmp/source.dart", "/tmp/source.dart"),
            ("file:///tmp", "/tmp"),
        ] {
            assert_eq!(
                resolve_file_reference(base, reference).unwrap(),
                PathBuf::from(expected)
            );
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_backslash_is_a_filename_character_but_a_remote_authority_is_not_local() {
        let base = Path::new("/workspace/config.json");
        assert_eq!(
            resolve_file_reference(base, "file:///tmp/a%5Cb.dart").unwrap(),
            PathBuf::from("/tmp/a\\b.dart")
        );
        assert!(resolve_file_reference(base, "file://server.invalid/share/file.dart").is_err());
    }
}
