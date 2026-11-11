use super::*;

/// Verified offsets can be moved with their owned backing buffer without self-references.
pub(super) struct Frame {
    /// Small postcard header remains raw until whole-file integrity is established.
    pub(super) header: Range<usize>,
    /// Every URI is unique and every record range is inside the checked body.
    pub(super) records: BTreeMap<String, Record>,
}

/// Check integrity first, then index lengths while skipping every typed AST.
pub(super) fn index(bytes: &[u8]) -> Option<Frame> {
    let end = bytes.len().checked_sub(8)?;
    let checksum = u64::from_le_bytes(bytes.get(end..)?.try_into().ok()?);
    let body = bytes.get(..end)?;
    if checksum != format::checksum(body) {
        return None;
    }
    let mut cursor = Cursor {
        bytes: body,
        offset: 0,
    };
    if cursor.take(format::MAGIC.len())? != format::MAGIC || cursor.u32()? != SCHEMA {
        return None;
    }
    let header = cursor.blob()?;
    let count = usize::try_from(cursor.u64()?).ok()?;
    // Even an empty record needs three length prefixes, options and a mode byte.
    if count > body.len().checked_sub(cursor.offset)? / 30 {
        return None;
    }
    let mut records = BTreeMap::new();
    for _ in 0..count {
        let uri = cursor.blob()?;
        let uri = std::str::from_utf8(body.get(uri)?).ok()?;
        if uri.is_empty() {
            return None;
        }
        let role = cursor.byte()?;
        if role != 0 {
            return None;
        }
        let options = (role, cursor.u16()?, cursor.u16()?);
        let full = match cursor.byte()? {
            0 => false,
            1 => true,
            _ => return None,
        };
        let text = cursor.blob()?;
        std::str::from_utf8(body.get(text.clone())?).ok()?;
        let library = cursor.blob()?;
        let record = Record {
            options,
            full,
            text,
            library,
        };
        if records.insert(uri.to_owned(), record).is_some() {
            return None;
        }
    }
    (cursor.offset == body.len()).then_some(Frame { header, records })
}

/// Checked offsets never advance beyond the body, including on hostile u64 lengths.
struct Cursor<'a> {
    /// Checksum trailer is excluded from this input slice.
    bytes: &'a [u8],
    /// Next unread framing byte.
    offset: usize,
}

impl<'a> Cursor<'a> {
    /// Reads exactly one field only after checked arithmetic and bounds validation.
    fn take(&mut self, length: usize) -> Option<&'a [u8]> {
        let end = self.offset.checked_add(length)?;
        let value = self.bytes.get(self.offset..end)?;
        self.offset = end;
        Some(value)
    }

    /// Reads the source role or full/namespace mode tag.
    fn byte(&mut self) -> Option<u8> {
        Some(*self.take(1)?.first()?)
    }

    /// Reads a fixed-width language-version component.
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }

    /// Reads the private schema without a typed metadata decode.
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    /// Reads a fixed-width count or length prefix.
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    /// Records a payload range and skips its bytes without decoding any AST.
    fn blob(&mut self) -> Option<Range<usize>> {
        let length = usize::try_from(self.u64()?).ok()?;
        let start = self.offset;
        self.take(length)?;
        Some(start..self.offset)
    }
}
