use super::*;

#[test]
fn an_earlier_head_uses_the_available_slot_without_discarding_cached_work() {
    let head = source("a-head");
    let cached = ["b", "c", "d", "e", "f", "g", "h"].map(source);
    let mut queue = cached
        .iter()
        .cloned()
        .map(|source| (source.uri.clone(), source))
        .collect::<BTreeMap<_, _>>();
    queue.insert(head.uri.clone(), head.clone());
    let (sender, jobs) = mpsc::channel();
    let (_notify, completed) = mpsc::sync_channel(2);
    let active = AtomicUsize::new(1);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: cached
            .iter()
            .map(|source| (source.uri.clone(), PendingParse::Ready(None)))
            .collect(),
        imports: HashSet::new(),
        known: cached.iter().map(|source| source.uri.clone()).collect(),
        speculative: BTreeMap::new(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: Some(source("z").uri),
        active: &active,
        active_limit: 2,
        limit: 8,
    };
    ahead.fill(&queue, &BTreeMap::new());
    assert_eq!(jobs.try_recv().unwrap().source.uri, head.uri);
    assert!(matches!(jobs.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert_eq!(ahead.pending.len(), ahead.limit);
    assert!(
        cached
            .iter()
            .all(|source| ahead.pending.contains_key(&source.uri))
    );
    assert!(ahead.pending.contains_key(&head.uri));
    assert_eq!(active.load(Relaxed), ahead.active_limit);
}

#[test]
fn a_ready_canonical_result_is_discarded_before_an_inflight_icon_parse() {
    let head = source("a-head");
    let cheap = source("b-cheap");
    let icons = source("icons");
    let (sender, _jobs) = mpsc::channel();
    let (_notify, completed) = mpsc::sync_channel(2);
    let (reply, response) = mpsc::channel();
    let active = AtomicUsize::new(1);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::from([
            (head.uri.clone(), PendingParse::Ready(None)),
            (cheap.uri.clone(), PendingParse::Ready(None)),
            (icons.uri.clone(), PendingParse::Waiting(response)),
        ]),
        imports: HashSet::new(),
        known: HashSet::new(),
        speculative: BTreeMap::new(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: None,
        active: &active,
        active_limit: 2,
        limit: 8,
    };
    assert!(ahead.discard_farthest(Some(&head.uri)));
    assert!(!ahead.pending.contains_key(&cheap.uri));
    assert!(ahead.pending.contains_key(&head.uri));
    assert!(ahead.pending.contains_key(&icons.uri));
    reply.send(None).unwrap();
}
