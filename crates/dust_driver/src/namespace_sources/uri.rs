use std::{fmt::Write, path::Path};

/// URI identity is distinct from a translated filesystem path.
pub(super) fn file_uri(path: &Path) -> Result<String, String> {
    let path = path.to_str().ok_or("source path is not valid UTF-8")?;
    #[cfg(windows)]
    let path = path.replace('\\', "/");
    let path = encode_path(&path.replace('%', "%25"))?;
    Ok(if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    })
}

/// Normalize escapes without conflating escaped separators or URI empty segments.
pub(super) fn encode_path(path: &str) -> Result<String, String> {
    let mut encoded = String::new();
    let mut bytes = path.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let hi = bytes.next().and_then(hex);
            let lo = bytes.next().and_then(hex);
            let value = hi
                .zip(lo)
                .map(|(hi, lo)| hi << 4 | lo)
                .ok_or("invalid percent escape in source URI")?;
            if unreserved(value) {
                encoded.push(char::from(value));
            } else {
                write!(encoded, "%{value:02X}").expect("string write");
            }
        } else if unreserved(byte) || b"!$&'()*+,;=:@/".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            write!(encoded, "%{byte:02X}").expect("string write");
        }
    }
    Ok(encoded)
}

/// Remove URI dot segments while retaining repeated slash identities.
pub(super) fn normalize(path: &str) -> String {
    let rooted = path.starts_with('/');
    let parts = path.split('/').collect::<Vec<_>>();
    let mut result = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        match *part {
            "." => {}
            ".." if result.len() > usize::from(rooted) && result.last() != Some(&"..") => {
                result.pop();
            }
            ".." if !rooted => result.push(".."),
            ".." => {}
            other => result.push(other),
        }
        if index + 1 == parts.len() && matches!(*part, "." | "..") {
            result.push("");
        }
    }
    result.join("/")
}

/// Package-relative resolution clamps parents inside that package's URI namespace.
pub(super) fn relative_path(base: &str, reference: &str) -> Result<String, String> {
    let encoded = encode_path(reference)?;
    let path = if encoded.starts_with('/') {
        encoded
    } else if reference.is_empty() {
        format!("/{base}")
    } else {
        match base.rsplit_once('/') {
            Some((parent, _)) => format!("/{parent}/{encoded}"),
            None => format!("/{encoded}"),
        }
    };
    Ok(normalize(&path)
        .strip_prefix('/')
        .unwrap_or(&path)
        .to_owned())
}

/// A file URI may be spelled differently from its translated path.
pub(super) fn file_reference(base: &str, reference: &str) -> Result<String, String> {
    let path = if let Some((scheme, value)) =
        reference.split_once(':').filter(|(s, _)| !s.contains('/'))
    {
        if !scheme.eq_ignore_ascii_case("file") {
            return Err(format!("unsupported source URI scheme `{scheme}`"));
        }
        if let Some(value) = value.strip_prefix("//") {
            let (authority, path) = value.split_once('/').unwrap_or((value, ""));
            if !authority.is_empty() {
                return Err(format!(
                    "unsupported source file URI authority `{authority}`"
                ));
            }
            format!("/{path}")
        } else {
            format!("/{}", value.trim_start_matches('/'))
        }
    } else if reference.starts_with('/') {
        reference.to_owned()
    } else if reference.is_empty() {
        return Ok(base.to_owned());
    } else {
        let parent = base
            .strip_prefix("file://")
            .ok_or("missing file URI base")?
            .rsplit_once('/')
            .map_or("", |(parent, _)| parent);
        format!("{parent}/{reference}")
    };
    Ok(format!("file://{}", normalize(&encode_path(&path)?)))
}

/// URI-unreserved bytes may retain their literal spelling in identity paths.
fn unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"-._~".contains(&byte)
}

/// Decodes one case-insensitive percent-escape digit.
fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
