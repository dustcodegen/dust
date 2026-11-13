use super::*;

impl Loader<'_> {
    /// Keeps speculative scheduling separate from canonical source ordering.
    pub(super) fn enqueue(&mut self, source: Source) {
        if !self.prepared_roots.contains(&source.uri) && !self.generated.contains_key(&source.uri) {
            self.prepare_queue
                .insert(source.uri.clone(), source.clone());
        } else {
            self.prepare_queue.remove(&source.uri);
        }
        self.queue.insert(source.uri.clone(), source);
    }

    /// A canonical turn removes its scheduling hint before live input validation.
    pub(super) fn pop_source(&mut self) -> Option<(String, Source)> {
        let next = self.queue.pop_first()?;
        self.prepare_queue.remove(&next.0);
        Some(next)
    }
}

/// Eligibility is URI-specific and changes when generated ownership is established.
#[cfg(test)]
mod tests;
