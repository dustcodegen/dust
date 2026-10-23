//! Finding Dust annotation names in Dart source without parsing it.

use super::*;

/// Returns whether source contains any supported annotation short name.
pub(super) fn has_supported_annotation(
    source: &str,
    supported_annotations: &SupportedAnnotations,
) -> bool {
    annotation_short_names(source)
        .into_iter()
        .any(|name| supported_annotations.contains(name))
}

/// Collects annotation short names, including the ones nested in an argument list.
///
/// The name written after the `@` is not always the one a plugin owns.
/// `@Derive([ToString(), FromRow()])` is how a row mapper is normally declared,
/// so a scan that reads `Derive` and stops leaves the file invisible to every
/// plugin whose annotation is inside the brackets — which is how a row class
/// stayed out of `dust db build` while a query was checked against it.
///
/// Inside the argument list only constructor calls count: an identifier
/// immediately followed by `(`. String literals are skipped, so the SQL handed
/// to `@Query` does not contribute `count` from `count(*)`.
pub(super) fn annotation_short_names(source: &str) -> Vec<&str> {
    // shortcut: lexical candidates only; replace with resolved graph discovery in #114.
    let mut names = Vec::new();
    for (index, _) in source.match_indices('@') {
        let Some((name, after)) = identifier_at(source, index + 1) else {
            continue;
        };
        names.push(name);
        if source[after..].starts_with('(') {
            collect_call_names(source, after, &mut names);
        }
    }
    names
}

/// Collects the names called inside one balanced annotation argument list.
pub(super) fn collect_call_names<'a>(source: &'a str, open: usize, names: &mut Vec<&'a str>) {
    let bytes = source.as_bytes();
    let mut depth = 0_usize;
    let mut index = open;
    while index < bytes.len() {
        index = skip_trivia(source, index);
        let Some(&byte) = bytes.get(index) else {
            return;
        };
        match byte {
            b'(' => {
                depth += 1;
                index += 1;
            }
            b')' => {
                if depth <= 1 {
                    return;
                }
                depth -= 1;
                index += 1;
            }
            b'\'' | b'"' => index = string_end(source, index),
            byte if byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$') => {
                let Some((name, after)) = identifier_at(source, index) else {
                    index += 1;
                    continue;
                };
                if source[after..].starts_with('(') {
                    names.push(name);
                }
                index = after;
            }
            _ => index += 1,
        }
    }
}

/// Returns the final qualified identifier and the first offset after its trivia.
pub(super) fn identifier_at(source: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = source.as_bytes();
    let mut start = skip_trivia(source, start);
    loop {
        let mut end = start;
        while bytes
            .get(end)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$'))
        {
            end += 1;
        }
        if end == start {
            return None;
        }
        let after = skip_trivia(source, end);
        if bytes.get(after) != Some(&b'.') {
            return Some((&source[start..end], after));
        }
        start = skip_trivia(source, after + 1);
    }
}

/// Returns the offset just past the string literal opening at `start`.
pub(super) fn string_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let quote = bytes[start];
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            byte if byte == quote => return index + 1,
            _ => index += 1,
        }
    }
    source.len()
}

/// Advances over Dart trivia without rewriting source or reading inside strings.
fn skip_trivia(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut index = start;
    loop {
        if bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        } else if bytes[index..].starts_with(b"//") {
            index = bytes[index..]
                .iter()
                .position(|&byte| byte == b'\n')
                .map_or(bytes.len(), |end| index + end + 1);
        } else if bytes[index..].starts_with(b"/*") {
            index = block_comment_end(bytes, index);
        } else {
            return index;
        }
    }
}

/// Dart block comments may nest; unfinished trivia consumes the remaining input.
fn block_comment_end(bytes: &[u8], start: usize) -> usize {
    let mut depth = 1;
    let mut index = start + 2;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"/*") {
            depth += 1;
            index += 2;
        } else if bytes[index..].starts_with(b"*/") {
            depth -= 1;
            index += 2;
            if depth == 0 {
                return index;
            }
        } else {
            index += 1;
        }
    }
    bytes.len()
}

/// Returns whether source imports or exports a Dust annotation package URI.
pub(super) fn contains_dust_annotation_package(source: &str) -> bool {
    DUST_RUNTIME_PACKAGES
        .iter()
        .any(|(_, import_uri)| source.contains(import_uri))
}

/// Returns Dust runtime package imports found in source.
pub(super) fn dust_package_imports(source: &str) -> Vec<&'static str> {
    DUST_RUNTIME_PACKAGES
        .iter()
        .filter_map(|(package, import_uri)| source.contains(import_uri).then_some(*package))
        .collect()
}
