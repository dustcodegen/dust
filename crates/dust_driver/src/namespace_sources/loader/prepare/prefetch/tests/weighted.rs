use super::*;

#[test]
fn larger_descendants_start_first_while_canonical_head_and_bounds_stay_intact() {
    let root = tempfile::tempdir().unwrap();
    let head = source("a-head");
    let parent = source("parent");
    let names = ["b-small", "y-large", "z-large"];
    let mut facts = prepared(
        &parent,
        "export 'b-small.dart'; export 'z-large.dart'; export 'y-large.dart';",
        &names,
    );
    for (name, bytes) in [("b-small", 1), ("y-large", 8), ("z-large", 8)] {
        let target = facts
            .targets
            .get_mut(&format!("{name}.dart"))
            .unwrap()
            .as_mut()
            .unwrap();
        target.path = root.path().join(format!("{name}.dart"));
        std::fs::write(&target.path, vec![b' '; bytes]).unwrap();
    }
    let queue = BTreeMap::from([(head.uri.clone(), head.clone())]);
    let (sender, jobs) = mpsc::channel();
    let (_notify, completed) = mpsc::sync_channel(4);
    let (head_reply, head_response) = mpsc::channel();
    let (observed, order) = mpsc::channel();
    let active = AtomicUsize::new(0);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::from([(head.uri.clone(), PendingParse::Waiting(head_response))]),
        imports: HashSet::new(),
        known: HashSet::from([head.uri.clone()]),
        speculative: Default::default(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: None,
        active: &active,
        active_limit: 2,
        limit: 6,
    };
    ahead.discover_children(&parent.uri, &facts, &BTreeMap::new());
    assert_eq!(ahead.speculative.len(), 3);

    thread::scope(|scope| {
        scope.spawn(move || {
            for _ in 0..2 {
                let job = jobs
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                observed.send(job.source.uri).unwrap();
                job.reply.send(None).unwrap();
            }
        });
        ahead.fill(&queue, &BTreeMap::new());
        assert_eq!(order.recv().unwrap(), source("y-large").uri);
        assert_eq!(order.recv().unwrap(), source("z-large").uri);
    });
    assert!(ahead.pending.contains_key(&head.uri));
    assert_eq!(queue.first_key_value().unwrap().0, &head.uri);
    assert!(ahead.speculative.contains_key(&source("b-small").uri));
    assert_eq!(active.load(Relaxed), ahead.active_limit);
    assert!(ahead.pending.len() <= ahead.limit);
    assert!(ahead.speculative_jobs.len() <= ahead.limit.saturating_sub(ahead.active_limit));
    assert!(ahead.speculative.len() <= ahead.limit);
    head_reply.send(None).unwrap();
    assert!(
        ahead
            .take(&head, None, FileId::new(7), &queue, &BTreeMap::new())
            .is_none()
    );
    assert!(!ahead.pending.contains_key(&head.uri));
}
