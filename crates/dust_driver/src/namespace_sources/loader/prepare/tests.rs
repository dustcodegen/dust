use super::*;
use dust_dart_syntax::DartLanguageVersion;

#[test]
fn prepared_facts_rebind_both_diagnostic_views_to_the_queue_id() {
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
    let expected = input.parse(&source, id);
    assert!(!expected.diagnostics.is_empty());
    assert!(!expected.namespace_diagnostics.is_empty());
    let prepared = PreparedParse {
        parsed: input.parse(&source, PREPARED_FILE_ID),
        input: SourceInput {
            text: input.text.clone(),
            owner: None,
            generated_library: false,
        },
    };
    assert_eq!(prepared.rebind(&source, &input, id), Some(expected));
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
        parsed: input.parse(source, PREPARED_FILE_ID),
        input: SourceInput {
            text: input.text.clone(),
            owner: None,
            generated_library: false,
        },
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
fn known_jobs_are_submitted_before_the_required_reply_arrives() {
    let later = Source {
        uri: "package:example/later.dart".to_owned(),
        path: PathBuf::new(),
        language_version: DartLanguageVersion::latest_known(),
    };
    let queue = BTreeMap::from([(later.uri.clone(), later)]);
    let (sender, jobs) = mpsc::channel::<ParseJob>();
    let (notify, completed) = mpsc::sync_channel(1);
    let (head_reply, response) = mpsc::channel();
    let active = AtomicUsize::new(0);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::new(),
        active: &active,
        active_limit: 1,
        limit: 2,
    };

    thread::scope(|scope| {
        scope.spawn(move || {
            let job = jobs
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            assert_eq!(job.source.uri, "package:example/later.dart");
            head_reply.send(None).unwrap();
            notify.try_send(()).unwrap();
        });
        assert!(ahead.wait_for(response, &queue, &BTreeMap::new()).is_none());
    });
    assert!(ahead.pending.contains_key("package:example/later.dart"));
}
