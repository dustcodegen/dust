use std::{fs, io::Read};

use super::*;

/// Regular-file reads remain disposable; all framing is validated before typed metadata.
pub(super) fn load(path: &Path, annotations: &[&str]) -> Option<Cache> {
    if !fs::symlink_metadata(path).ok()?.file_type().is_file() {
        return None;
    }
    let mut file = fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).ok()?;
    let frame = frame::index(&contents)?;
    let header: format::Header = format::decode(contents.get(frame.header)?)?;
    if header.identity != crate::build::parser_identity()
        || !header
            .annotations
            .iter()
            .map(String::as_str)
            .eq(annotations.iter().copied())
    {
        return None;
    }
    Some(Cache {
        contents,
        records: frame.records,
        changed: Default::default(),
    })
}
