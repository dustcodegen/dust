use super::*;

#[test]
fn malformed_truncated_and_checksum_corrupt_frames_are_misses() {
    let fixture = Fixture::new(MODEL);
    let original = fs::read(fixture.path()).unwrap();
    for bytes in [
        b"not a cache".as_slice(),
        &original[..original.len() / 2],
        &original[..7],
    ] {
        fs::write(fixture.path(), bytes).unwrap();
        assert!(fixture.lookup().is_none());
    }
    let mut bad_checksum = original.clone();
    *bad_checksum.last_mut().unwrap() ^= 1;
    fs::write(fixture.path(), bad_checksum).unwrap();
    assert!(fixture.lookup().is_none());
    let mut wrong_magic = original;
    wrong_magic[0] ^= 1;
    reseal(&mut wrong_magic);
    fs::write(fixture.path(), wrong_magic).unwrap();
    assert!(fixture.lookup().is_none());
}

#[test]
fn overflowing_lengths_impossible_counts_and_trailing_bytes_are_rejected() {
    let fixture = Fixture::new(MODEL);
    let original = fs::read(fixture.path()).unwrap();
    let checked = frame::index(&original).unwrap();
    let record = &checked.records[&fixture.source.uri];
    let count = checked.header.end;
    let role = record.text.start - 14;
    let uri_length = role - fixture.source.uri.len() - 8;
    for offset in [
        12,
        count,
        uri_length,
        record.text.start - 8,
        record.library.start - 8,
    ] {
        let mut bytes = original.clone();
        bytes[offset..offset + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        reseal(&mut bytes);
        assert!(frame::index(&bytes).is_none(), "offset={offset}");
        fs::write(fixture.path(), bytes).unwrap();
        assert!(fixture.lookup().is_none());
    }
    let mut fewer_records = original.clone();
    fewer_records[count..count + 8].copy_from_slice(&0_u64.to_le_bytes());
    reseal(&mut fewer_records);
    assert!(frame::index(&fewer_records).is_none());
    let mut trailing = original;
    trailing.insert(trailing.len() - 8, 0);
    reseal(&mut trailing);
    assert!(frame::index(&trailing).is_none());
}

#[test]
fn duplicate_uris_invalid_flags_and_invalid_utf8_invalidate_framing() {
    let fixture = Fixture::new(MODEL);
    let original = fs::read(fixture.path()).unwrap();
    fixture.rewrite(|_, records| {
        let duplicate = root_record(records, &fixture.source.uri).clone();
        records.push(duplicate);
    });
    assert!(fixture.lookup().is_none());
    let checked = frame::index(&original).unwrap();
    let record = &checked.records[&fixture.source.uri];
    let role = record.text.start - 14;
    let mode = record.text.start - 9;
    let uri = role - fixture.source.uri.len();
    for (offset, invalid) in [
        (role, 99),
        (mode, 2),
        (uri, 0xff),
        (record.text.start, 0xff),
    ] {
        let mut bytes = original.clone();
        bytes[offset] = invalid;
        reseal(&mut bytes);
        assert!(frame::index(&bytes).is_none(), "offset={offset}");
        fs::write(fixture.path(), bytes).unwrap();
        assert!(fixture.lookup().is_none());
    }
}

#[test]
fn header_blobs_cannot_contain_a_second_postcard_value() {
    let fixture = Fixture::new(MODEL);
    let mut bytes = fs::read(fixture.path()).unwrap();
    let checked = frame::index(&bytes).unwrap();
    let length = checked.header.len() as u64 + 1;
    bytes.insert(checked.header.end, 0);
    bytes[12..20].copy_from_slice(&length.to_le_bytes());
    reseal(&mut bytes);
    assert!(frame::index(&bytes).is_some(), "framing remains valid");
    fs::write(fixture.path(), bytes).unwrap();
    assert!(
        fixture.lookup().is_none(),
        "header decoding must reject a remainder"
    );
}
