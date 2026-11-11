use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
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

use super::{ParsePolicy, ResolvedTargets, Source, SourceIndex, SourceInput, resolve_targets};

/// Shared construction and exact rebinding of immutable prepared facts.
mod facts;
/// Completed facts may prepare descendants without changing canonical discovery.
mod prefetch;
/// Scoped workers prepare immutable parser and directive facts.
mod worker;

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
pub(super) struct PreparedParse {
    /// Exact input used by the worker.
    input: SourceInput,
    /// Original parser facts with provisional diagnostic labels.
    parsed: ParseResult,
    /// Immutable directive identities derived from those exact parser facts.
    targets: ResolvedTargets,
}

/// Canonical units and independently cached roots need no speculative preparation.
#[derive(Default)]
pub(super) struct KnownSources {
    /// Sources already retained as canonical units.
    pub(super) loaded: HashSet<String>,
    /// URI-keyed root facts retained outside the parser pool.
    pub(super) cached: HashSet<String>,
}

/// Retained work counts against one bound before and after worker completion.
enum PendingParse {
    /// The worker owns its result until the coordinator harvests it.
    Waiting(mpsc::Receiver<Option<PreparedParse>>),
    /// Completed facts remain speculative until their canonical queue turn.
    Ready(Option<Box<PreparedParse>>),
}

/// Bounded parsing of known queue entries. Only the loader consumes them.
pub(super) struct ParseAhead<'a> {
    /// Sends new requests to the scoped pool.
    sender: mpsc::Sender<ParseJob>,
    /// Completed URIs let the coordinator harvest any ready parent during a wait.
    completed: mpsc::Receiver<String>,
    /// In-flight requests and completed results awaiting their queue turn.
    pending: BTreeMap<String, PendingParse>,
    /// Import eligibility captured once for this canonical drain.
    imports: HashSet<String>,
    /// Loaded, consumed or previously submitted identities prevent speculative cycles.
    known: HashSet<String>,
    /// Cached roots are skipped by canonical dispatch as well as speculation.
    cached_roots: HashSet<String>,
    /// Bounded descendant identities awaiting speculative worker slots.
    speculative: BTreeMap<String, Source>,
    /// Retained speculative requests cannot fill the canonical result window.
    speculative_jobs: HashSet<String>,
    /// Last visited URI in the current refill range.
    cursor: Option<String>,
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
        index: &SourceIndex,
        potential: &BTreeSet<PathBuf>,
        policy: &ParsePolicy,
        imports: HashSet<String>,
        known: KnownSources,
        consume: impl FnOnce(Option<&mut ParseAhead<'_>>) -> T,
    ) -> T {
        if workers == 1 {
            return consume(None);
        }
        let (sender, receiver) = mpsc::channel::<ParseJob>();
        let (notify, completed) = mpsc::sync_channel(workers * 2);
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
                    .spawn_scoped(scope, move || {
                        worker::run(&receiver, index, potential, policy, active, &notify)
                    })
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
            let KnownSources { mut loaded, cached } = known;
            loaded.extend(cached.iter().cloned());
            let mut ahead = ParseAhead {
                sender,
                completed,
                pending: BTreeMap::new(),
                imports,
                known: loaded,
                cached_roots: cached,
                speculative: BTreeMap::new(),
                speculative_jobs: HashSet::new(),
                cursor: None,
                active: &active,
                active_limit: started * 2,
                // Prepared facts move into Units, so the census bounds this frontier.
                limit: policy.root_count().max(started * 8),
            };
            consume(Some(&mut ahead))
        })
    }

    /// Starts preparing the next queue entries, without reserving any file IDs.
    pub(super) fn fill(
        &mut self,
        queued: &BTreeMap<String, Source>,
        generated: &BTreeMap<String, String>,
    ) {
        self.fill_queue(queued, generated, false);
    }

    /// Fills available slots without waiting for a particular source.
    fn fill_queue(
        &mut self,
        queued: &BTreeMap<String, Source>,
        generated: &BTreeMap<String, String>,
        waiting_for_head: bool,
    ) {
        self.pump_completed(generated);
        // The removed head still counts against the retained result limit.
        let limit = self.limit - usize::from(waiting_for_head);
        // New earlier dependencies need one slot, not another full worker window.
        let priority = queued.first_key_value().is_some_and(|(uri, _)| {
            self.cursor.as_ref().is_some_and(|cursor| uri < cursor)
                && !self.pending.contains_key(uri)
                && !generated.contains_key(uri)
                && !self.cached_roots.contains(uri)
        });
        if priority && let Some((uri, _)) = queued.first_key_value() {
            if self.pending.len() >= limit {
                self.discard_farthest(Some(uri));
            }
            self.cursor = None;
        }
        if !priority {
            self.fill_speculative(queued, generated, limit);
        }
        self.fill_canonical(queued, generated, limit);
        if priority {
            self.fill_speculative(queued, generated, limit);
        }
    }

    /// Uses a prepared parse only when its input matches the current source.
    pub(super) fn take(
        &mut self,
        source: &Source,
        input: Option<&SourceInput>,
        id: FileId,
        queue: &BTreeMap<String, Source>,
        generated: &BTreeMap<String, String>,
    ) -> Option<(ParseResult, ResolvedTargets)> {
        self.known.insert(source.uri.clone());
        self.speculative.remove(&source.uri);
        self.speculative_jobs.remove(&source.uri);
        let response = self.pending.remove(&source.uri)?;
        let input = input?;
        if input.owner.is_some() {
            return None;
        }
        let prepared = match response {
            PendingParse::Ready(prepared) => prepared.map(|prepared| *prepared),
            PendingParse::Waiting(response) => self.wait_for(response, queue, generated),
        }?;
        self.discover_children(&source.uri, &prepared, generated);
        prepared.rebind(source, input, id)
    }

    /// Waits for the required URI while keeping the known queue fed.
    fn wait_for(
        &mut self,
        response: mpsc::Receiver<Option<PreparedParse>>,
        queue: &BTreeMap<String, Source>,
        generated: &BTreeMap<String, String>,
    ) -> Option<PreparedParse> {
        loop {
            self.fill_queue(queue, generated, true);
            match response.try_recv() {
                Ok(prepared) => return prepared,
                Err(mpsc::TryRecvError::Disconnected) => return None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
            match self.completed.recv() {
                Ok(uri) => self.harvest(&uri, generated),
                Err(_) => return response.recv().ok().flatten(),
            }
        }
    }
}

/// Parser facts can be reused only for the exact source snapshot.
#[cfg(test)]
mod tests;
