use super::*;

/// Checked lengths, counts, flags and UTF-8 precede typed postcard decoding.
mod framing;

#[test]
fn a_header_cannot_be_relabelled_with_another_valid_identity_or_policy() {
    let fixture = Fixture::new(MODEL);
    let (header, records) = fixture.snapshot();
    for mode in 0..3 {
        let mut wrong_header = header.clone();
        match mode {
            0 => wrong_header.identity ^= 1,
            1 => wrong_header.annotations = vec!["DifferentAnnotation".into()],
            _ => {}
        }
        let mut differently_bound = encode(&wrong_header, &records);
        if mode == 2 {
            differently_bound[8..12].copy_from_slice(&(SCHEMA ^ 1).to_le_bytes());
            reseal(&mut differently_bound);
        }
        // Restore matching metadata while retaining the other header's checksum.
        let mut relabelled = encode(&header, &records);
        let end = relabelled.len() - 8;
        let other_end = differently_bound.len() - 8;
        relabelled[end..].copy_from_slice(&differently_bound[other_end..]);
        fs::write(fixture.path(), relabelled).unwrap();
        assert!(fixture.lookup().is_none(), "{mode}");
    }
}

#[test]
fn metadata_mode_and_language_options_must_match_before_ast_reuse() {
    let fixture = Fixture::new(MODEL);
    let original = fs::read(fixture.path()).unwrap();
    for mode in 0..3 {
        fs::write(fixture.path(), &original).unwrap();
        fixture.rewrite(|_, records| {
            let record = root_record(records, &fixture.source.uri);
            match mode {
                0 => record.full = false,
                1 => record.options.2 = 12,
                _ => record.text = MODEL.replace("Model", "Other").into_bytes(),
            }
        });
        assert!(fixture.lookup().is_none(), "{mode}");
    }
}

#[test]
fn malformed_truncated_and_trailing_ast_blobs_are_lazy_misses() {
    let fixture = Fixture::new(MODEL);
    let original = fs::read(fixture.path()).unwrap();
    for mode in 0..3 {
        fs::write(fixture.path(), &original).unwrap();
        fixture.rewrite(|_, records| {
            let record = root_record(records, &fixture.source.uri);
            match mode {
                0 => record.library.clear(),
                1 => {
                    record.library.pop();
                }
                _ => record.library.push(0),
            }
        });
        assert!(frame::index(&fs::read(fixture.path()).unwrap()).is_some());
        let cache = fixture.cache();
        assert!(!cache.needs_save(), "framing alone does not decode ASTs");
        assert!(
            cache
                .lookup(&fixture.source, MODEL, fixture.options, true)
                .is_none()
        );
        assert!(
            !cache.needs_save(),
            "decoding failure is not a successful native parse"
        );
    }
}

#[test]
fn resealed_reversed_out_of_source_and_overflowing_ast_ranges_are_misses() {
    let fixture = Fixture::new(MODEL);
    let original = fs::read(fixture.path()).unwrap();
    for mode in 0..3 {
        fs::write(fixture.path(), &original).unwrap();
        fixture.rewrite(|_, records| {
            let record = root_record(records, &fixture.source.uri);
            match mode {
                0 => {
                    let mut library = fixture.fresh(FileId::new(0)).library;
                    library.span = dust_text::TextRange::new(0_u32, (MODEL.len() + 1) as u32);
                    record.library = postcard::to_stdvec(&library).unwrap();
                }
                1 => {
                    // The first field is TextRange; checked decoding locates its tail.
                    let (_, tail) =
                        postcard::take_from_bytes::<(u32, u32)>(&record.library).unwrap();
                    let mut invalid = postcard::to_stdvec(&(9_u32, 0_u32)).unwrap();
                    invalid.extend_from_slice(tail);
                    record.library = invalid;
                }
                _ => {
                    let (_, tail) = postcard::take_from_bytes::<u32>(&record.library).unwrap();
                    // 2^32 would become the original zero start if a decoder truncated it.
                    let mut invalid = vec![0x80, 0x80, 0x80, 0x80, 0x10];
                    invalid.extend_from_slice(tail);
                    record.library = invalid;
                }
            }
        });
        assert!(fixture.lookup().is_none(), "{mode}");
    }
    for overflow in [
        [0xff, 0xff, 0xff, 0xff, 0x1f],
        [0x80, 0x80, 0x80, 0x80, 0x10],
    ] {
        assert!(format::decode::<dust_text::TextSize>(&overflow).is_none());
    }
}

#[test]
fn a_resealed_program_span_cannot_split_a_utf8_character() {
    let text = "class Model { final String text = '🦀'; }";
    let fixture = Fixture::new(text);
    assert!(fixture.lookup().is_some());
    let interior = text.find('🦀').unwrap() + 1;
    let mut library = fixture.fresh(FileId::new(0)).library;
    library.span = dust_text::TextRange::new(interior, text.len());
    fixture.rewrite(|_, records| {
        root_record(records, &fixture.source.uri).library = postcard::to_stdvec(&library).unwrap();
    });
    assert!(fixture.lookup().is_none());
}
