use std::{collections::hash_map::DefaultHasher, hash::Hasher};

use serde::{Deserialize, Serialize};

use super::*;

/// Fixed signature distinguishes this private binary snapshot from all earlier formats.
pub(super) const MAGIC: &[u8; 8] = b"DUSTPARS";

/// All ParseOptions fields participate without changing their public serialization API.
pub(super) type Options = (u8, u16, u16);

/// Parser identity and exact namespace projection policy are covered by the file checksum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Header {
    /// Parser implementation, dependency and target identity.
    pub(super) identity: u64,
    /// Ordered supported-annotation names used by namespace projection.
    pub(super) annotations: Vec<String>,
}

/// Every codec blob must contain exactly one value; postcard otherwise permits a remainder.
pub(super) fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Option<T> {
    let (value, rest) = postcard::take_from_bytes(bytes).ok()?;
    rest.is_empty().then_some(value)
}

/// Adding a source role forces its wire identity to be considered explicitly.
pub(super) fn options(options: ParseOptions) -> Options {
    let role = match options.source_kind {
        SourceKind::Library => 0,
    };
    (
        role,
        options.language_version.major,
        options.language_version.minor,
    )
}

/// Small header allocation is independent of the borrowed parser surfaces.
pub(super) fn begin(header: &Header, count: usize) -> Result<Vec<u8>, postcard::Error> {
    let header = postcard::to_stdvec(header)?;
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&SCHEMA.to_le_bytes());
    append_blob(&mut bytes, &header);
    bytes.extend_from_slice(&(count as u64).to_le_bytes());
    Ok(bytes)
}

/// A record copies original source and one temporary AST blob into the final buffer.
pub(super) fn append_record(
    bytes: &mut Vec<u8>,
    uri: &str,
    options: Options,
    full: bool,
    text: &[u8],
    library: &[u8],
) {
    append_blob(bytes, uri.as_bytes());
    bytes.push(options.0);
    bytes.extend_from_slice(&options.1.to_le_bytes());
    bytes.extend_from_slice(&options.2.to_le_bytes());
    bytes.push(u8::from(full));
    append_blob(bytes, text);
    append_blob(bytes, library);
}

/// All metadata, framing and AST/source bytes precede this integrity trailer.
pub(super) fn seal(bytes: &mut Vec<u8>) {
    let checksum = checksum(bytes);
    bytes.extend_from_slice(&checksum.to_le_bytes());
}

/// Rust-version changes may discard this optional cache; no stable digest contract is needed.
pub(super) fn checksum(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

/// Length prefixes permit source and AST indexing without interpreting either payload.
fn append_blob(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value);
}
