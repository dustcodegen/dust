use std::fs;

use super::*;

/// Releases the active slot and wakes the coordinator even on worker unwind.
struct Completion<'a> {
    /// The required URI's reply must close before the wake is sent.
    reply: Option<mpsc::Sender<Option<PreparedParse>>>,
    /// Shared worker notification channel, which can hold one outstanding wake.
    notify: &'a mpsc::SyncSender<String>,
    /// A completion notice identifies its own per-job reply, including failures.
    uri: String,
    /// Worker slots currently in use.
    active: &'a AtomicUsize,
}

impl Drop for Completion<'_> {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Relaxed);
        drop(self.reply.take());
        let _ = self.notify.send(self.uri.clone());
    }
}

/// Prepares immutable facts, leaving live input validation and errors to the loader.
pub(super) fn run(
    receiver: &Mutex<mpsc::Receiver<ParseJob>>,
    index: &SourceIndex,
    potential: &BTreeSet<PathBuf>,
    policy: &ParsePolicy,
    active: &AtomicUsize,
    notify: &mpsc::SyncSender<String>,
) {
    loop {
        let job = receiver.lock().expect("parser job lock").recv();
        let Ok(ParseJob { source, reply }) = job else {
            return;
        };
        let completion = Completion {
            reply: Some(reply),
            notify,
            active,
            uri: source.uri.clone(),
        };
        let prepared = policy
            .preloaded_input(&source)
            .or_else(|| {
                // Speculation must not wait on a device or named pipe in the queue.
                fs::metadata(&source.path)
                    .ok()
                    .filter(|metadata| metadata.is_file())
                    .and_then(|_| SourceInput::read(&source, None, potential).ok())
            })
            .map(|input| PreparedParse::new(index, &source, input, policy));
        let _ = completion
            .reply
            .as_ref()
            .expect("active parser reply")
            .send(prepared);
    }
}
