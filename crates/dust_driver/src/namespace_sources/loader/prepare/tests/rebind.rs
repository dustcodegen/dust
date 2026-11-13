use super::*;

#[test]
fn prepared_facts_rebind_both_diagnostic_views_and_retain_targets() {
    let source = Source {
        uri: "package:example/bad.dart".to_owned(),
        path: PathBuf::new(),
        language_version: DartLanguageVersion::latest_known(),
    };
    let input = SourceInput {
        text: "class Bad {".into(),
        owner: None,
        generated_library: false,
    };
    let id = FileId::new(3);
    let expected = input.parse(&source, id, &ParsePolicy::default());
    assert!(!expected.diagnostics.is_empty());
    assert!(!expected.namespace_diagnostics.is_empty());
    let prepared = PreparedParse {
        parsed: input.parse(&source, PREPARED_FILE_ID, &ParsePolicy::default()),
        input: SourceInput {
            text: input.text.clone(),
            owner: None,
            generated_library: false,
        },
        targets: BTreeMap::from([("foreign:bad".into(), Err("unsupported URI".into()))]),
    };
    let (parsed, targets) = prepared.rebind(&source, &input, id).unwrap();
    assert_eq!(parsed, expected);
    assert_eq!(
        targets["foreign:bad"].as_ref().unwrap_err(),
        "unsupported URI"
    );
}

#[test]
fn prepared_facts_are_discarded_when_input_or_language_changes() {
    let mut source = Source {
        uri: "package:example/model.dart".to_owned(),
        path: PathBuf::new(),
        language_version: DartLanguageVersion::latest_known(),
    };
    let input = SourceInput {
        text: "class Model {}".into(),
        owner: None,
        generated_library: false,
    };
    let prepare = |source: &Source| PreparedParse {
        parsed: input.parse(source, PREPARED_FILE_ID, &ParsePolicy::default()),
        input: SourceInput {
            text: input.text.clone(),
            owner: None,
            generated_library: false,
        },
        targets: BTreeMap::new(),
    };
    let changed = SourceInput {
        text: "class Other {}".into(),
        owner: None,
        generated_library: false,
    };
    assert!(
        prepare(&source)
            .rebind(&source, &changed, FileId::new(4))
            .is_none()
    );
    let parsed = prepare(&source);
    source.language_version = DartLanguageVersion::DART_3_0;
    assert!(parsed.rebind(&source, &input, FileId::new(4)).is_none());
}

#[test]
fn a_late_generated_owner_discards_prepared_facts_without_waiting() {
    let source = Source {
        uri: "package:example/model.g.dart".into(),
        path: PathBuf::new(),
        language_version: DartLanguageVersion::latest_known(),
    };
    let input = SourceInput {
        text: "".into(),
        owner: Some("package:example/model.dart".into()),
        generated_library: false,
    };
    let (sender, _jobs) = mpsc::channel();
    let (_notify, completed) = mpsc::sync_channel(1);
    let (reply, response) = mpsc::channel();
    let active = AtomicUsize::new(0);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::from([(source.uri.clone(), PendingParse::Waiting(response))]),
        imports: HashSet::new(),
        known: HashSet::new(),
        speculative: Default::default(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: None,
        active: &active,
        active_limit: 1,
        limit: 2,
    };
    assert!(
        ahead
            .take(
                &source,
                Some(&input),
                FileId::new(2),
                &BTreeMap::new(),
                &BTreeMap::new()
            )
            .is_none()
    );
    assert!(reply.send(None).is_err());
}
