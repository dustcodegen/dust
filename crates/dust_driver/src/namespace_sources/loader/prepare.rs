use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering::Relaxed},
        mpsc,
    },
    thread,
};

use dust_parser_dart::ParseResult;
use dust_text::FileId;

use super::{Source, SourceInput};

/// Speculative parse results use a neutral ID until the queue consumes them.
const PREPARED_FILE_ID: FileId = FileId::new(0);

/// A queued source and the channel for its speculative result.
struct ParseJob {
    /// Identity, physical path, and language version to read.
    source: Source,
    /// Worker response, empty when preparation could not read a regular file.
    reply: mpsc::Sender<Option<PreparedParse>>,
}

/// One parse tied to the source snapshot the worker read.
struct PreparedParse {
    /// Exact input used by the worker.
    input: SourceInput,
    /// Original parser facts with provisional diagnostic labels.
    parsed: ParseResult,
}

/// Bounded parsing of known queue entries. Only the loader consumes them.
pub(super) struct ParseAhead<'a> {
    /// Sends new requests to the scoped pool.
    sender: mpsc::Sender<ParseJob>,
    /// Coalesced worker completions that let the coordinator refill during a wait.
    completed: mpsc::Receiver<()>,
    /// In-flight requests and completed results awaiting their queue turn.
    pending: BTreeMap<String, mpsc::Receiver<Option<PreparedParse>>>,
    /// Counts requests still queued or being parsed.
    active: &'a AtomicUsize,
    /// Limits the amount of unfinished work ahead of newly queued sources.
    active_limit: usize,
    /// Caps the number of prepared results retained across queue changes.
    limit: usize,
}

impl ParseAhead<'_> {
    /// Runs the queue coordinator with scoped parser workers when available.
    pub(super) fn run<T>(
        workers: usize,
        potential: &BTreeSet<PathBuf>,
        consume: impl FnOnce(Option<&mut ParseAhead<'_>>) -> T,
    ) -> T {
        if workers == 1 {
            return consume(None);
        }
        let (sender, receiver) = mpsc::channel::<ParseJob>();
        let (notify, completed) = mpsc::sync_channel(1);
        let receiver = Arc::new(Mutex::new(receiver));
        let active = AtomicUsize::new(0);
        thread::scope(|scope| {
            let mut started = 0;
            for _ in 0..workers {
                let receiver = Arc::clone(&receiver);
                let notify = notify.clone();
                let active = &active;
                // Parser extraction walks nested Dart syntax recursively.
                if thread::Builder::new()
                    .stack_size(8 * 1024 * 1024)
                    .spawn_scoped(scope, move || worker(&receiver, potential, active, &notify))
                    .is_ok()
                {
                    started += 1;
                }
            }
            // Dropping the last worker also closes replies for queued jobs.
            drop(receiver);
            drop(notify);
            if started == 0 {
                return consume(None);
            }
            let mut ahead = ParseAhead {
                sender,
                completed,
                pending: BTreeMap::new(),
                active: &active,
                active_limit: started * 2,
                limit: started * 8,
            };
            consume(Some(&mut ahead))
        })
    }

    /// Starts preparing the next queue entries, without reserving any file IDs.
    pub(super) fn fill<'a>(
        &mut self,
        queued: impl Iterator<Item = (&'a String, &'a Source)>,
        generated: &BTreeMap<String, String>,
    ) {
        self.fill_queue(queued, generated, false);
    }

    /// Fills available slots and reports whether completion could free more.
    fn fill_queue<'a>(
        &mut self,
        queued: impl Iterator<Item = (&'a String, &'a Source)>,
        generated: &BTreeMap<String, String>,
        waiting_for_head: bool,
    ) -> bool {
        // The removed head still counts against the retained and lookahead limits.
        let limit = self.limit - usize::from(waiting_for_head);
        let skip_first = !waiting_for_head && !self.pending.is_empty();
        for (index, (uri, source)) in queued.take(limit).enumerate() {
            if self.pending.len() >= limit {
                break;
            }
            // A newly discovered first URI can parse locally without waiting
            // behind earlier work submitted to the pool.
            if (index == 0 && skip_first)
                || generated.contains_key(uri)
                || self.pending.contains_key(uri)
            {
                continue;
            }
            if self.active.load(Relaxed) >= self.active_limit {
                return true;
            }
            let (reply, receiver) = mpsc::channel();
            self.active.fetch_add(1, Relaxed);
            if self
                .sender
                .send(ParseJob {
                    source: source.clone(),
                    reply,
                })
                .is_err()
            {
                self.active.fetch_sub(1, Relaxed);
                break;
            }
            self.pending.insert(uri.clone(), receiver);
        }
        false
    }

    /// Uses a prepared parse only when its input matches the current source.
    pub(super) fn take(
        &mut self,
        source: &Source,
        input: Option<&SourceInput>,
        id: FileId,
        queue: &BTreeMap<String, Source>,
        generated: &BTreeMap<String, String>,
    ) -> Option<ParseResult> {
        let response = self.pending.remove(&source.uri)?;
        let input = input?;
        if input.owner.is_some() {
            return None;
        }
        self.wait_for(response, queue, generated)?
            .rebind(source, input, id)
    }

    /// Waits for the required URI while keeping the known queue fed.
    fn wait_for(
        &mut self,
        response: mpsc::Receiver<Option<PreparedParse>>,
        queue: &BTreeMap<String, Source>,
        generated: &BTreeMap<String, String>,
    ) -> Option<PreparedParse> {
        loop {
            match response.try_recv() {
                Ok(prepared) => return prepared,
                Err(mpsc::TryRecvError::Disconnected) => return None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
            if !self.fill_queue(queue.iter(), generated, true) || self.completed.recv().is_err() {
                return response.recv().ok().flatten();
            }
        }
    }
}

impl PreparedParse {
    /// Validates the input and installs the canonical ID on both diagnostic views.
    fn rebind(self, source: &Source, input: &SourceInput, id: FileId) -> Option<ParseResult> {
        let Self {
            input: before,
            mut parsed,
        } = self;
        if before != *input || parsed.options != SourceInput::options(source) {
            return None;
        }
        for diagnostic in parsed
            .diagnostics
            .iter_mut()
            .chain(&mut parsed.namespace_diagnostics)
        {
            for label in &mut diagnostic.labels {
                label.file_id = id;
            }
        }
        Some(parsed)
    }
}

/// Releases the active slot and wakes the coordinator even on worker unwind.
struct Completion<'a> {
    /// The required URI's reply must close before the wake is sent.
    reply: Option<mpsc::Sender<Option<PreparedParse>>>,
    /// Shared worker notification channel, which can hold one outstanding wake.
    notify: &'a mpsc::SyncSender<()>,
    /// Worker slots currently in use.
    active: &'a AtomicUsize,
}

impl Drop for Completion<'_> {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Relaxed);
        drop(self.reply.take());
        let _ = self.notify.try_send(());
    }
}

/// Prepares only parser facts, leaving live source errors to the loader.
fn worker(
    receiver: &Mutex<mpsc::Receiver<ParseJob>>,
    potential: &BTreeSet<PathBuf>,
    active: &AtomicUsize,
    notify: &mpsc::SyncSender<()>,
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
        };
        // Speculation must not wait on a device or named pipe in the queue.
        let prepared = fs::metadata(&source.path)
            .ok()
            .filter(|metadata| metadata.is_file())
            .and_then(|_| SourceInput::read(&source, None, potential).ok())
            .map(|input| PreparedParse {
                parsed: input.parse(&source, PREPARED_FILE_ID),
                input,
            });
        let _ = completion
            .reply
            .as_ref()
            .expect("active parser reply")
            .send(prepared);
    }
}

/// Parser facts can be reused only for the exact source snapshot.
#[cfg(test)]
mod tests;
