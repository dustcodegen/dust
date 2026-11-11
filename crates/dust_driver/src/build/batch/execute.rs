use crate::build::{
    batch::ProgressReporter,
    process::{IndexedBuildOutcome, PendingLibrary, ProcessingConfig, process_pending_library},
};

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

/// Processes pending libraries on the current thread in discovery order.
pub(super) fn process_pending_serial(
    pending: Vec<PendingLibrary>,
    fail_fast: bool,
    processing: &ProcessingConfig<'_>,
    reporter: &ProgressReporter<'_>,
) -> Vec<IndexedBuildOutcome> {
    let mut processed = Vec::with_capacity(pending.len());

    for pending in pending {
        let outcome = process_pending_library(pending, processing, reporter);
        let has_error = outcome
            .outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.is_error());
        processed.push(outcome);

        if fail_fast && has_error {
            break;
        }
    }

    processed
}

/// Processes pending libraries across scoped worker threads.
pub(super) fn process_pending_parallel(
    pending: Vec<PendingLibrary>,
    jobs: usize,
    fail_fast: bool,
    processing: &ProcessingConfig<'_>,
    reporter: &ProgressReporter<'_>,
) -> Vec<IndexedBuildOutcome> {
    let jobs = jobs.max(1).min(pending.len());
    let pending = Mutex::new(pending.into_iter());
    let stop = Arc::new(AtomicBool::new(false));

    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(jobs);
        for _ in 0..jobs {
            let pending = &pending;
            let stop = Arc::clone(&stop);
            handles.push(scope.spawn(move || {
                let mut processed = Vec::new();
                loop {
                    if fail_fast && stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let next = pending.lock().expect("job queue must not poison").next();
                    let Some(pending) = next else {
                        break;
                    };
                    let outcome = process_pending_library(pending, processing, reporter);
                    if fail_fast
                        && outcome
                            .outcome
                            .diagnostics
                            .iter()
                            .any(|diagnostic| diagnostic.is_error())
                    {
                        stop.store(true, Ordering::Relaxed);
                    }
                    processed.push(outcome);
                }
                processed
            }));
        }

        let mut processed = Vec::new();
        for handle in handles {
            processed.extend(handle.join().expect("worker thread must not panic"));
        }
        processed
    })
}
