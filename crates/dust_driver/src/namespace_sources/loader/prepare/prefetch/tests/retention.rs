use super::*;

#[test]
fn canonical_results_are_evicted_before_speculative_descendants() {
    let head = source("a-head");
    let expensive = source("z-expensive");
    let (sender, _jobs) = mpsc::channel();
    let (_notify, completed) = mpsc::sync_channel(2);
    let active = AtomicUsize::new(0);
    let mut pending = ["a-head", "b-root", "c-root"]
        .map(source)
        .into_iter()
        .map(|source| (source.uri, PendingParse::Ready(None)))
        .collect::<BTreeMap<_, _>>();
    pending.insert(
        expensive.uri.clone(),
        PendingParse::Ready(Some(Box::new(prepared(
            &expensive,
            "class Expensive {}",
            &[],
        )))),
    );
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending,
        imports: HashSet::new(),
        known: HashSet::new(),
        speculative: BTreeMap::new(),
        speculative_jobs: HashSet::from([expensive.uri.clone()]),
        cached_roots: HashSet::new(),
        cursor: None,
        active: &active,
        active_limit: 2,
        limit: 8,
    };

    assert!(ahead.discard_farthest(Some(&head.uri)));
    assert!(!ahead.pending.contains_key(&source("c-root").uri));
    assert!(ahead.pending.contains_key(&source("b-root").uri));
    assert!(ahead.pending.contains_key(&expensive.uri));
    assert!(ahead.discard_farthest(Some(&head.uri)));
    assert!(!ahead.pending.contains_key(&source("b-root").uri));
    assert!(ahead.pending.contains_key(&expensive.uri));
    assert!(ahead.discard_farthest(Some(&head.uri)));
    assert!(!ahead.pending.contains_key(&expensive.uri));
    assert!(!ahead.speculative_jobs.contains(&expensive.uri));
    assert!(ahead.pending.contains_key(&head.uri));
    assert!(!ahead.discard_farthest(Some(&head.uri)));
}

#[test]
fn ready_speculation_can_exceed_the_worker_window_within_total_retention() {
    let queue = ["a-head", "b-root"]
        .map(source)
        .into_iter()
        .map(|source| (source.uri.clone(), source))
        .collect::<BTreeMap<_, _>>();
    let (sender, jobs) = mpsc::channel();
    let (_notify, completed) = mpsc::sync_channel(2);
    let active = AtomicUsize::new(0);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: queue
            .keys()
            .map(|uri| (uri.clone(), PendingParse::Ready(None)))
            .collect(),
        imports: HashSet::new(),
        known: queue.keys().cloned().collect(),
        speculative: (0..7)
            .map(|index| source(&format!("child{index}")))
            .map(|source| (source.uri.clone(), source))
            .collect(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: None,
        active: &active,
        active_limit: 2,
        limit: 8,
    };
    for _ in 0..3 {
        ahead.fill_speculative(&queue, &BTreeMap::new(), ahead.limit);
        for _ in 0..ahead.active_limit {
            let job = jobs.try_recv().unwrap();
            let uri = job.source.uri.clone();
            job.reply
                .send(Some(prepared(&job.source, "class Child {}", &[])))
                .unwrap();
            ahead.harvest(&uri, &BTreeMap::new());
        }
        // The controlled worker replies have completed; their ASTs stay retained.
        active.store(0, Relaxed);
    }
    assert_eq!(
        ahead.speculative_jobs.len(),
        ahead.limit - ahead.active_limit
    );
    assert!(ahead.speculative_jobs.len() > ahead.active_limit);
    assert_eq!(ahead.pending.len(), ahead.limit);
    assert!(queue.keys().all(|uri| ahead.pending.contains_key(uri)));
    ahead.fill_speculative(&queue, &BTreeMap::new(), ahead.limit);
    assert!(matches!(jobs.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert_eq!(ahead.speculative.len(), 1);
    assert_eq!(active.load(Relaxed), 0);
}

#[test]
fn scanning_a_future_cached_result_keeps_its_speculative_eviction_priority() {
    let head = source("a-head");
    let root = source("b-root");
    let future = source("z-icons");
    let queue = [head.clone(), root.clone(), future.clone()]
        .into_iter()
        .map(|source| (source.uri.clone(), source))
        .collect::<BTreeMap<_, _>>();
    let (sender, _jobs) = mpsc::channel();
    let (_notify, completed) = mpsc::sync_channel(2);
    let active = AtomicUsize::new(0);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::from([
            (head.uri.clone(), PendingParse::Ready(None)),
            (root.uri.clone(), PendingParse::Ready(None)),
            (
                future.uri.clone(),
                PendingParse::Ready(Some(Box::new(prepared(&future, "class Icons {}", &[])))),
            ),
        ]),
        imports: HashSet::new(),
        known: queue.keys().cloned().collect(),
        speculative: BTreeMap::new(),
        speculative_jobs: HashSet::from([future.uri.clone()]),
        cached_roots: HashSet::new(),
        cursor: None,
        active: &active,
        active_limit: 2,
        limit: 8,
    };

    ahead.fill_canonical(&queue, &BTreeMap::new(), ahead.limit);
    assert_eq!(ahead.cursor.as_ref(), Some(&future.uri));
    assert!(ahead.speculative_jobs.contains(&future.uri));
    assert!(ahead.discard_farthest(Some(&head.uri)));
    assert!(!ahead.pending.contains_key(&root.uri));
    assert!(ahead.pending.contains_key(&future.uri));

    // Merely reaching the next queue entry must not demote it before consumption.
    ahead.pending.remove(&head.uri);
    ahead.fill(
        &BTreeMap::from([(future.uri.clone(), future.clone())]),
        &BTreeMap::new(),
    );
    assert!(ahead.speculative_jobs.contains(&future.uri));
    assert!(ahead.pending.contains_key(&future.uri));
    let input = SourceInput {
        text: "class Icons {}".into(),
        owner: None,
        generated_library: false,
    };
    assert!(
        ahead
            .take(
                &future,
                Some(&input),
                FileId::new(9),
                &BTreeMap::new(),
                &BTreeMap::new()
            )
            .is_some()
    );
    assert!(!ahead.speculative_jobs.contains(&future.uri));
    assert!(!ahead.pending.contains_key(&future.uri));
}

#[test]
fn waiting_for_the_current_head_does_not_demote_the_next_future() {
    let future = source("z-icons");
    let queue = BTreeMap::from([(future.uri.clone(), future.clone())]);
    let (sender, _jobs) = mpsc::channel();
    let (_notify, completed) = mpsc::sync_channel(2);
    let (reply, response) = mpsc::channel();
    let active = AtomicUsize::new(0);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::from([(
            future.uri.clone(),
            PendingParse::Ready(Some(Box::new(prepared(&future, "class Icons {}", &[])))),
        )]),
        imports: HashSet::new(),
        known: HashSet::from([future.uri.clone()]),
        speculative: BTreeMap::new(),
        speculative_jobs: HashSet::from([future.uri.clone()]),
        cached_roots: HashSet::new(),
        cursor: Some(future.uri.clone()),
        active: &active,
        active_limit: 2,
        limit: 8,
    };
    reply.send(None).unwrap();
    assert!(ahead.wait_for(response, &queue, &BTreeMap::new()).is_none());
    assert!(ahead.speculative_jobs.contains(&future.uri));
    assert!(ahead.pending.contains_key(&future.uri));
}
