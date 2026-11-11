use super::*;

#[test]
fn exact_facts_equal_fresh_parses_with_neutral_and_current_file_ids() {
    let fixture = Fixture::new(MODEL);
    let cached = fixture.lookup().expect("saved handwritten parser facts");
    for id in [FileId::new(0), FileId::new(87)] {
        let fresh = fixture.fresh(id);
        assert!(fresh.diagnostics.is_empty());
        assert_eq!(cached, fresh);
    }
    assert!(cached.diagnostics.is_empty());
    assert!(cached.namespace_diagnostics.is_empty());
    assert_eq!(cached.library.classes[0].fields.len(), 1);
    assert_eq!(cached.library.classes[0].methods.len(), 1);
}

#[test]
fn source_trivia_and_trailing_newlines_keep_exact_cached_program_spans() {
    for text in [
        format!("\n  // note\n{MODEL}\n\n"),
        format!("/* 🦀 */\n{MODEL} // tail\n"),
    ] {
        let fixture = Fixture::new(&text);
        let cached = fixture
            .lookup()
            .expect("trivia must not prevent exact parser reuse");
        assert_eq!(cached, fixture.fresh(FileId::new(88)));
    }
}

#[test]
fn exact_source_uri_options_and_root_mode_are_independent_identity_inputs() {
    let fixture = Fixture::new(MODEL);
    let cache = fixture.cache();
    assert!(
        cache
            .lookup(&fixture.source, MODEL, fixture.options, true)
            .is_some()
    );
    for text in [MODEL.replace("Model", "Other"), format!("{MODEL}\n")] {
        assert!(
            cache
                .lookup(&fixture.source, &text, fixture.options, true)
                .is_none()
        );
    }
    assert_eq!(MODEL.len(), MODEL.replace("Model", "Other").len());
    let mut alias = fixture.source.clone();
    alias.uri = "package:parser_cache_test/source.dart".into();
    assert!(cache.lookup(&alias, MODEL, fixture.options, true).is_none());
    let changed = ParseOptions {
        language_version: DartLanguageVersion::DART_3_12,
        ..fixture.options
    };
    assert!(
        cache
            .lookup(&fixture.source, MODEL, changed, true)
            .is_none()
    );
    assert!(
        cache
            .lookup(&fixture.source, MODEL, fixture.options, false)
            .is_none()
    );
}

#[test]
fn parser_identity_schema_and_annotation_policy_mismatches_are_misses() {
    let fixture = Fixture::new(MODEL);
    let original = fs::read(fixture.path()).unwrap();
    let mut wrong_schema = original.clone();
    wrong_schema[8..12].copy_from_slice(&(SCHEMA ^ 1).to_le_bytes());
    reseal(&mut wrong_schema);
    fs::write(fixture.path(), wrong_schema).unwrap();
    assert!(fixture.lookup().is_none());
    fs::write(fixture.path(), &original).unwrap();
    fixture.rewrite(|header, _| header.identity ^= 1);
    assert!(fixture.lookup().is_none());
    fs::write(fixture.path(), original).unwrap();
    assert!(
        Cache::load(fixture.root.path(), &["DifferentAnnotation"])
            .lookup(&fixture.source, MODEL, fixture.options, true)
            .is_none()
    );
    fixture.rewrite(|header, _| header.annotations = vec!["ChangedAnnotation".into()]);
    assert!(fixture.lookup().is_none());
}

#[test]
fn external_projection_cannot_be_reused_as_a_complete_root_surface() {
    let fixture = Fixture::new("import 'package:collection/collection.dart'; class Root {}");
    let index = index(fixture.root.path());
    let external = index
        .resolve(&fixture.source, "package:collection/collection.dart")
        .unwrap();
    fs::write(&external.path, MODEL).unwrap();
    fixture.save();
    let options = ParseOptions {
        language_version: external.language_version,
        ..Default::default()
    };
    let cache = fixture.cache();
    let projected = cache.lookup(&external, MODEL, options, false).unwrap();
    assert_eq!(projected.library.classes[0].name, "Model");
    assert!(projected.library.classes[0].fields.is_empty());
    assert!(projected.library.classes[0].methods.is_empty());
    assert!(cache.lookup(&external, MODEL, options, true).is_none());
}

#[test]
fn only_successful_native_parse_notifications_require_snapshot_replacement() {
    let fixture = Fixture::new(MODEL);
    let cache = fixture.cache();
    assert!(!cache.needs_save());
    assert!(
        cache
            .lookup(&fixture.source, MODEL, fixture.options, true)
            .is_some()
    );
    assert!(!cache.needs_save());
    for mode in 0..3 {
        let cache = fixture.cache();
        let mut source = fixture.source.clone();
        let mut text = MODEL;
        let mut full = true;
        match mode {
            0 => source.uri = "file:///missing.dart".into(),
            1 => text = "class Other {}",
            _ => full = false,
        }
        assert!(cache.lookup(&source, text, fixture.options, full).is_none());
        assert!(
            !cache.needs_save(),
            "lookup miss {mode} alone cannot justify a rewrite"
        );
    }
    fixture.rewrite(|_, records| root_record(records, &fixture.source.uri).library.clear());
    let cache = fixture.cache();
    assert!(!cache.needs_save(), "typed surfaces decode lazily");
    assert!(
        cache
            .lookup(&fixture.source, MODEL, fixture.options, true)
            .is_none()
    );
    assert!(
        !cache.needs_save(),
        "a corrupt entry alone cannot justify a rewrite"
    );
    let parsed = fixture.fresh(FileId::new(92));
    assert!(parsed.diagnostics.is_empty());
    assert!(parsed.namespace_diagnostics.is_empty());
    cache.note_parsed();
    assert!(cache.needs_save());
}
