use dust_dart_syntax::DartLanguageVersion;

use super::*;

/// New canonical heads reserve only needed capacity and preserve active work.
mod priority;
/// Retention prioritizes speculative descendants without growing either bound.
mod retention;

fn source(name: &str) -> Source {
    Source {
        uri: format!("package:example/{name}.dart"),
        path: PathBuf::new(),
        language_version: DartLanguageVersion::latest_known(),
    }
}

fn prepared(parent: &Source, text: &str, names: &[&str]) -> PreparedParse {
    let input = SourceInput {
        text: text.into(),
        owner: None,
        generated_library: false,
    };
    let parsed = input.parse(parent, PREPARED_FILE_ID, &ParsePolicy::default());
    let targets = names
        .iter()
        .map(|name| (format!("{name}.dart"), Ok(source(name))))
        .collect();
    PreparedParse {
        input,
        parsed,
        targets,
    }
}

#[test]
fn a_ready_parent_prepares_its_child_before_the_delayed_head_and_stays_bounded() {
    let head = source("a-head");
    let parent = source("b-parent");
    let child = source("grandchild");
    let queue = ["b-parent", "c", "d", "e", "f", "g", "h"]
        .map(source)
        .into_iter()
        .map(|source| (source.uri.clone(), source))
        .collect::<BTreeMap<_, _>>();
    let (sender, jobs) = mpsc::channel();
    let (notify, completed) = mpsc::sync_channel(2);
    let (parent_reply, parent_response) = mpsc::channel();
    let (head_reply, head_response) = mpsc::channel();
    let mut pending = queue
        .keys()
        .map(|uri| (uri.clone(), PendingParse::Ready(None)))
        .collect::<BTreeMap<_, _>>();
    pending.insert(parent.uri.clone(), PendingParse::Waiting(parent_response));
    let active = AtomicUsize::new(1);
    let mut known = queue.keys().cloned().collect::<HashSet<_>>();
    known.insert(head.uri.clone());
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending,
        imports: HashSet::from([parent.uri.clone()]),
        known,
        speculative: BTreeMap::new(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: Some(source("z").uri),
        active: &active,
        active_limit: 2,
        limit: 8,
    };
    let facts = prepared(&parent, "import 'grandchild.dart';", &["grandchild"]);
    let parent_uri = parent.uri.clone();
    let head_uri = head.uri.clone();
    let child_uri = child.uri.clone();
    thread::scope(|scope| {
        scope.spawn(move || {
            parent_reply.send(Some(facts)).unwrap();
            notify.send(parent_uri).unwrap();
            let job = jobs
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            assert_eq!(job.source.uri, child_uri);
            head_reply.send(None).unwrap();
            notify.send(head_uri).unwrap();
        });
        assert!(
            ahead
                .wait_for(head_response, &queue, &BTreeMap::new())
                .is_none()
        );
    });
    assert!(ahead.pending.contains_key(&parent.uri));
    assert!(ahead.pending.contains_key(&child.uri));
    assert_eq!(ahead.pending.len(), ahead.limit - 1);
    assert!(ahead.speculative_jobs.len() <= ahead.limit.saturating_sub(ahead.active_limit));
    assert!(ahead.speculative.len() <= ahead.limit);
    assert!(active.load(Relaxed) <= ahead.active_limit);
    assert!(!queue.contains_key(&child.uri));

    let changed = SourceInput {
        text: "class Parent {}".into(),
        owner: None,
        generated_library: false,
    };
    assert!(
        ahead
            .take(
                &parent,
                Some(&changed),
                FileId::new(4),
                &queue,
                &BTreeMap::new()
            )
            .is_none()
    );
    assert!(!queue.contains_key(&child.uri));
    assert!(ahead.pending.contains_key(&child.uri));
}

#[test]
fn speculation_follows_only_eligible_edges_and_keeps_its_frontier_bounded() {
    let parent = source("parent");
    let (sender, _jobs) = mpsc::channel();
    let (_notify, completed) = mpsc::sync_channel(2);
    let (reply, response) = mpsc::channel();
    let active = AtomicUsize::new(0);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::from([(parent.uri.clone(), PendingParse::Waiting(response))]),
        imports: HashSet::new(),
        known: HashSet::from([parent.uri.clone(), source("loaded").uri]),
        speculative: BTreeMap::new(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: None,
        active: &active,
        active_limit: 2,
        limit: 4,
    };
    reply
        .send(Some(prepared(
            &parent,
            "import 'ignored.dart'; export 'needed.dart'; export 'loaded.dart'; part 'piece.dart';",
            &["ignored", "needed", "loaded", "piece"],
        )))
        .unwrap();
    ahead.harvest(&parent.uri, &BTreeMap::new());
    assert_eq!(
        ahead.speculative.keys().cloned().collect::<Vec<_>>(),
        [source("needed").uri, source("piece").uri]
    );

    let names = ["one", "two", "three", "four", "five"];
    let text = names
        .iter()
        .map(|name| format!("export '{name}.dart';"))
        .collect::<String>();
    ahead.speculative.clear();
    ahead.limit = 2;
    let (reply, response) = mpsc::channel();
    ahead
        .pending
        .insert(parent.uri.clone(), PendingParse::Waiting(response));
    reply.send(Some(prepared(&parent, &text, &names))).unwrap();
    ahead.harvest(&parent.uri, &BTreeMap::new());
    assert_eq!(ahead.speculative.len(), ahead.limit);
}

#[test]
fn a_direct_head_reply_prepares_an_already_queued_future_before_intervening_roots() {
    let head = source("a-head");
    let near = source("b-near");
    let future = source("z-future");
    let queue = ["b-near", "c", "d", "e", "z-future"]
        .map(source)
        .into_iter()
        .map(|source| (source.uri.clone(), source))
        .collect::<BTreeMap<_, _>>();
    let (sender, jobs) = mpsc::channel();
    let (_notify, completed) = mpsc::sync_channel(2);
    let (reply, response) = mpsc::channel();
    let active = AtomicUsize::new(2);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::from([
            (head.uri.clone(), PendingParse::Waiting(response)),
            (near.uri.clone(), PendingParse::Ready(None)),
        ]),
        imports: HashSet::from([head.uri.clone()]),
        known: HashSet::from([head.uri.clone(), near.uri.clone()]),
        speculative: BTreeMap::new(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: Some(near.uri.clone()),
        active: &active,
        active_limit: 2,
        limit: 8,
    };
    let facts = prepared(&head, "import 'z-future.dart';", &["z-future"]);
    let live_input = SourceInput {
        text: Arc::clone(&facts.input.text),
        owner: None,
        generated_library: false,
    };
    reply.send(Some(facts)).unwrap();
    assert!(
        ahead
            .take(
                &head,
                Some(&live_input),
                FileId::new(1),
                &queue,
                &BTreeMap::new()
            )
            .is_some()
    );
    assert!(ahead.speculative.contains_key(&future.uri));
    assert!(matches!(jobs.try_recv(), Err(mpsc::TryRecvError::Empty)));

    active.store(0, Relaxed);
    ahead.fill(&queue, &BTreeMap::new());
    assert_eq!(jobs.try_recv().unwrap().source.uri, future.uri);
    assert_eq!(jobs.try_recv().unwrap().source.uri, source("c").uri);
    assert!(ahead.speculative_jobs.contains(&future.uri));
    assert!(ahead.pending.contains_key(&near.uri));
    assert_eq!(queue.first_key_value().unwrap().0, &near.uri);
    assert!(ahead.pending.len() <= ahead.limit);
    assert!(active.load(Relaxed) <= ahead.active_limit);
}
