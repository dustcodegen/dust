use dust_parser_dart::ParsedDirective;
use std::ops::Bound::{Excluded, Unbounded};

use super::super::{implicit_core, literals};
use super::*;

impl ParseAhead<'_> {
    /// Canonical requests retain their existing URI frontier and worker bound.
    pub(super) fn fill_canonical(
        &mut self,
        queued: &BTreeMap<String, Source>,
        generated: &BTreeMap<String, String>,
        limit: usize,
    ) {
        if self.pending.len() >= limit {
            return;
        }
        if self.active.load(Relaxed) >= self.active_limit {
            return;
        }
        let after = self.cursor.clone().map_or(Unbounded, Excluded);
        for (uri, source) in queued.range((after, Unbounded)) {
            if self.pending.len() >= limit {
                break;
            }
            if generated.contains_key(uri)
                || self.pending.contains_key(uri)
                || self.cached_roots.contains(uri)
            {
                self.cursor = Some(uri.clone());
                continue;
            }
            if self.active.load(Relaxed) >= self.active_limit {
                return;
            }
            if !self.submit(source, false) {
                break;
            }
            self.cursor = Some(uri.clone());
        }
    }

    /// Notices are bounded and harvesting never dispatches recursively.
    pub(super) fn pump_completed(&mut self, generated: &BTreeMap<String, String>) {
        while let Ok(uri) = self.completed.try_recv() {
            self.harvest(&uri, generated);
        }
    }

    /// Stores any ready parent and discovers children without committing a Unit.
    pub(super) fn harvest(&mut self, uri: &str, generated: &BTreeMap<String, String>) {
        let Some(pending) = self.pending.remove(uri) else {
            return;
        };
        let prepared = match pending {
            PendingParse::Ready(prepared) => {
                self.pending
                    .insert(uri.to_owned(), PendingParse::Ready(prepared));
                return;
            }
            PendingParse::Waiting(response) => match response.try_recv() {
                Ok(prepared) => prepared,
                Err(mpsc::TryRecvError::Disconnected) => None,
                Err(mpsc::TryRecvError::Empty) => {
                    self.pending
                        .insert(uri.to_owned(), PendingParse::Waiting(response));
                    return;
                }
            },
        };
        if let Some(prepared) = &prepared {
            self.discover_children(uri, prepared, generated);
        }
        self.pending
            .insert(uri.to_owned(), PendingParse::Ready(prepared.map(Box::new)));
    }

    /// Reuses the same structured edges; only already eligible imports are followed.
    pub(super) fn discover_children(
        &mut self,
        uri: &str,
        prepared: &PreparedParse,
        generated: &BTreeMap<String, String>,
    ) {
        let core = implicit_core(&prepared.parsed.library.directives, uri);
        for directive in prepared
            .parsed
            .library
            .directives
            .iter()
            .chain(core.as_ref())
        {
            if matches!(directive, ParsedDirective::Import { .. }) && !self.imports.contains(uri)
                || matches!(directive, ParsedDirective::PartOf { .. })
            {
                continue;
            }
            for literal in literals(directive) {
                let Some(reference) = literal.value() else {
                    continue;
                };
                let Some(Ok(source)) = prepared.targets.get(&reference) else {
                    continue;
                };
                if self.known.contains(&source.uri) || generated.contains_key(&source.uri) {
                    continue;
                }
                if self.speculative.len() >= self.limit {
                    return;
                }
                self.speculative.insert(source);
            }
        }
    }

    /// Both job classes share the unfinished-work and retained-result bounds.
    pub(super) fn submit(&mut self, source: &Source, speculative: bool) -> bool {
        let (reply, response) = mpsc::channel();
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
            return false;
        }
        self.known.insert(source.uri.clone());
        self.pending
            .insert(source.uri.clone(), PendingParse::Waiting(response));
        if speculative {
            self.speculative_jobs.insert(source.uri.clone());
        }
        true
    }

    /// Prefer replaceable canonical work while retaining the next canonical head.
    pub(super) fn discard_farthest(&mut self, protected: Option<&str>) -> bool {
        // Reverse order keeps the farthest URI within each retention class.
        let Some(uri) = self
            .pending
            .iter()
            .rev()
            .filter(|(uri, _)| Some(uri.as_str()) != protected)
            .min_by_key(|(uri, pending)| {
                match (self.speculative_jobs.contains(uri.as_str()), pending) {
                    (false, PendingParse::Ready(_)) => 0,
                    (false, PendingParse::Waiting(_)) => 1,
                    (true, PendingParse::Ready(_)) => 2,
                    (true, PendingParse::Waiting(_)) => 3,
                }
            })
            .map(|(uri, _)| uri.clone())
        else {
            return false;
        };
        self.pending.remove(&uri);
        self.speculative_jobs.remove(&uri);
        true
    }

    /// A bounded speculative window cannot monopolize the retained canonical cache.
    pub(super) fn fill_speculative(
        &mut self,
        queued: &BTreeMap<String, Source>,
        generated: &BTreeMap<String, String>,
        limit: usize,
    ) {
        let protected = queued.first_key_value().map(|(uri, _)| uri.as_str());
        let speculative_limit = limit.saturating_sub(self.active_limit);
        while self.speculative_jobs.len() < speculative_limit
            && self.active.load(Relaxed) < self.active_limit
        {
            let Some((bytes, source)) = self.speculative.pop_first() else {
                break;
            };
            if self.known.contains(&source.uri)
                || generated.contains_key(&source.uri)
                || self.cached_roots.contains(&source.uri)
                || Some(source.uri.as_str()) == protected
            {
                continue;
            }
            if self.pending.len() >= limit && !self.discard_farthest(protected) {
                self.speculative.restore((bytes, source));
                break;
            }
            if !self.submit(&source, true) {
                break;
            }
        }
    }
}

/// Controlled replies prove descendant overlap without sleeps or timing assumptions.
#[cfg(test)]
mod tests;
