use super::*;
use dust_dart_syntax::DartLanguageVersion;

/// Retention scales to the finite workspace census while active work stays bounded.
mod census;
/// Exact source validation and canonical diagnostics for prepared facts.
mod rebind;

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
        imports: HashSet::new(),
        known: HashSet::new(),
        speculative: Default::default(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: Some("package:example/z.dart".into()),
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
            notify.try_send(job.source.uri).unwrap();
        });
        assert!(ahead.wait_for(response, &queue, &BTreeMap::new()).is_none());
    });
    assert!(ahead.pending.contains_key("package:example/later.dart"));
}

#[test]
fn refills_prepare_earlier_discoveries_and_reuse_pending_roots() {
    let source = |name: &str| Source {
        uri: format!("package:example/{name}.dart"),
        path: PathBuf::new(),
        language_version: DartLanguageVersion::latest_known(),
    };
    let mut queue = ["b", "c", "d"]
        .map(source)
        .into_iter()
        .map(|source| (source.uri.clone(), source))
        .collect::<BTreeMap<_, _>>();
    let (sender, jobs) = mpsc::channel::<ParseJob>();
    let (_notify, completed) = mpsc::sync_channel(1);
    let active = AtomicUsize::new(0);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::new(),
        imports: HashSet::new(),
        known: HashSet::new(),
        speculative: Default::default(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: None,
        active: &active,
        active_limit: 2,
        limit: 4,
    };

    ahead.fill(&queue, &BTreeMap::new());
    for name in ["b", "c"] {
        assert_eq!(jobs.try_recv().unwrap().source.uri, source(name).uri);
    }
    ahead.pending.remove(&source("b").uri);
    queue.remove(&source("b").uri);
    active.store(0, Relaxed);
    let earlier = source("a");
    queue.insert(earlier.uri.clone(), earlier.clone());

    ahead.fill(&queue, &BTreeMap::new());
    assert_eq!(jobs.try_recv().unwrap().source.uri, source("a").uri);
    assert_eq!(jobs.try_recv().unwrap().source.uri, source("d").uri);
    assert_eq!(ahead.pending.len(), 3);
    assert_eq!(ahead.cursor.as_ref(), Some(&source("d").uri));
    assert!(ahead.pending.contains_key(&earlier.uri));
    ahead.fill(&queue, &BTreeMap::new());
    assert!(matches!(jobs.try_recv(), Err(mpsc::TryRecvError::Empty)));
}

#[test]
fn busy_slots_preserve_the_next_candidate_after_a_generated_entry() {
    let source = |name: &str| Source {
        uri: format!("package:example/{name}.dart"),
        path: PathBuf::new(),
        language_version: DartLanguageVersion::latest_known(),
    };
    let mut queue = ["b", "c", "d"]
        .map(source)
        .into_iter()
        .map(|source| (source.uri.clone(), source))
        .collect::<BTreeMap<_, _>>();
    let generated = BTreeMap::from([(source("c").uri, "owner".into())]);
    let (sender, jobs) = mpsc::channel::<ParseJob>();
    let (_notify, completed) = mpsc::sync_channel(1);
    let active = AtomicUsize::new(0);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::new(),
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

    ahead.fill(&queue, &generated);
    assert_eq!(jobs.try_recv().unwrap().source.uri, source("b").uri);
    assert_eq!(ahead.cursor.as_ref(), Some(&source("c").uri));
    ahead.fill(&queue, &generated);
    assert!(matches!(jobs.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert_eq!(ahead.cursor.as_ref(), Some(&source("c").uri));

    ahead.pending.remove(&source("b").uri);
    queue.remove(&source("b").uri);
    active.store(0, Relaxed);
    ahead.fill(&queue, &generated);
    assert_eq!(jobs.try_recv().unwrap().source.uri, source("d").uri);
    assert_eq!(ahead.cursor.as_ref(), Some(&source("d").uri));
    assert!(!ahead.pending.contains_key(&source("c").uri));
}

#[test]
fn earlier_dependency_siblings_use_workers_without_resubmitting_pending_roots() {
    let source = |name: &str| Source {
        uri: format!("package:example/{name}.dart"),
        path: PathBuf::new(),
        language_version: DartLanguageVersion::latest_known(),
    };
    let mut queue = ["m", "n"]
        .map(source)
        .into_iter()
        .map(|source| (source.uri.clone(), source))
        .collect::<BTreeMap<_, _>>();
    let (sender, jobs) = mpsc::channel::<ParseJob>();
    let (_notify, completed) = mpsc::sync_channel(1);
    let active = AtomicUsize::new(0);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::new(),
        imports: HashSet::new(),
        known: HashSet::new(),
        speculative: Default::default(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: None,
        active: &active,
        active_limit: 2,
        limit: 4,
    };

    ahead.fill(&queue, &BTreeMap::new());
    for name in ["m", "n"] {
        assert_eq!(jobs.try_recv().unwrap().source.uri, source(name).uri);
    }
    ahead.pending.remove(&source("m").uri);
    queue.remove(&source("m").uri);
    active.store(0, Relaxed);
    for name in ["a", "b"] {
        let source = source(name);
        queue.insert(source.uri.clone(), source);
    }

    ahead.fill(&queue, &BTreeMap::new());
    for name in ["a", "b"] {
        assert_eq!(jobs.try_recv().unwrap().source.uri, source(name).uri);
    }
    assert!(ahead.pending.contains_key(&source("a").uri));
    assert_eq!(ahead.pending.len(), 3);
    assert!(ahead.pending.contains_key(&source("n").uri));
    assert!(matches!(jobs.try_recv(), Err(mpsc::TryRecvError::Empty)));
}

#[test]
fn an_earlier_dependency_displaces_one_retained_root_and_evicted_work_returns_later() {
    let source = |name: &str| Source {
        uri: format!("package:example/{name}.dart"),
        path: PathBuf::new(),
        language_version: DartLanguageVersion::latest_known(),
    };
    let mut queue = (0..9)
        .map(|index| source(&format!("m{index}")))
        .map(|source| (source.uri.clone(), source))
        .collect::<BTreeMap<_, _>>();
    let (sender, jobs) = mpsc::channel::<ParseJob>();
    let (_notify, completed) = mpsc::sync_channel(1);
    let active = AtomicUsize::new(0);
    let mut ahead = ParseAhead {
        sender,
        completed,
        pending: BTreeMap::new(),
        imports: HashSet::new(),
        known: HashSet::new(),
        speculative: Default::default(),
        speculative_jobs: HashSet::new(),
        cached_roots: HashSet::new(),
        cursor: None,
        active: &active,
        active_limit: 2,
        limit: 8,
    };
    for _ in 0..4 {
        ahead.fill(&queue, &BTreeMap::new());
        active.store(0, Relaxed);
    }
    assert_eq!(ahead.pending.len(), ahead.limit);
    for index in 0..8 {
        assert_eq!(
            jobs.try_recv().unwrap().source.uri,
            source(&format!("m{index}")).uri
        );
    }
    for name in ["a", "b", "c"] {
        let source = source(name);
        queue.insert(source.uri.clone(), source);
    }

    ahead.fill(&queue, &BTreeMap::new());
    assert_eq!(jobs.try_recv().unwrap().source.uri, source("a").uri);
    assert!(matches!(jobs.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert_eq!(ahead.pending.len(), ahead.limit);
    assert!(ahead.pending.contains_key(&source("a").uri));
    assert!(ahead.pending.contains_key(&source("m6").uri));
    assert!(!ahead.pending.contains_key(&source("m7").uri));
    queue.retain(|uri, _| uri >= &source("m6").uri);
    ahead.pending.clear();
    active.store(0, Relaxed);
    ahead.fill(&queue, &BTreeMap::new());
    for name in ["m6", "m7"] {
        assert_eq!(jobs.try_recv().unwrap().source.uri, source(name).uri);
    }
}
