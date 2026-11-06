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
        let receiver = Arc::new(Mutex::new(receiver));
        let active = AtomicUsize::new(0);
        thread::scope(|scope| {
            let mut started = 0;
            for _ in 0..workers {
                let receiver = Arc::clone(&receiver);
                let active = &active;
                // Parser extraction walks nested Dart syntax recursively.
                if thread::Builder::new()
                    .stack_size(8 * 1024 * 1024)
                    .spawn_scoped(scope, move || worker(&receiver, potential, active))
                    .is_ok()
                {
                    started += 1;
                }
            }
            // Dropping the last worker also closes replies for queued jobs.
            drop(receiver);
            if started == 0 {
                return consume(None);
            }
            let mut ahead = ParseAhead {
                sender,
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
        let already_preparing = !self.pending.is_empty();
        for (index, (uri, source)) in queued.take(self.limit).enumerate() {
            if self.pending.len() == self.limit || self.active.load(Relaxed) == self.active_limit {
                break;
            }
            // A newly discovered first URI can parse locally without waiting
            // behind earlier work submitted to the pool.
            if (index == 0 && already_preparing)
                || generated.contains_key(uri)
                || self.pending.contains_key(uri)
            {
                continue;
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
    }

    /// Uses a prepared parse only when its input matches the current source.
    pub(super) fn take(
        &mut self,
        source: &Source,
        input: Option<&SourceInput>,
        id: FileId,
    ) -> Option<ParseResult> {
        let response = self.pending.remove(&source.uri)?;
        let input = input?;
        if input.owner.is_some() {
            return None;
        }
        response.recv().ok()??.rebind(source, input, id)
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

/// Prepares only parser facts, leaving live source errors to the loader.
fn worker(
    receiver: &Mutex<mpsc::Receiver<ParseJob>>,
    potential: &BTreeSet<PathBuf>,
    active: &AtomicUsize,
) {
    loop {
        let job = receiver.lock().expect("parser job lock").recv();
        let Ok(ParseJob { source, reply }) = job else {
            return;
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
        active.fetch_sub(1, Relaxed);
        let _ = reply.send(prepared);
    }
}

/// Parser facts can be reused only for the exact source snapshot.
#[cfg(test)]
mod tests {
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
}
