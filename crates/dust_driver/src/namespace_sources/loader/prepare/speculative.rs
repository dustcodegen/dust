use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    fs,
};

use super::Source;

/// Exact URI removal and largest-first preparation share the same bounded entries.
#[derive(Default)]
pub(super) struct Queue {
    /// Metadata is only a scheduling hint; canonical reads remain authoritative.
    entries: BTreeMap<String, (u64, Source)>,
    /// Equal file lengths retain the original lexical URI preparation order.
    priority: BTreeSet<(Reverse<u64>, String)>,
}

impl Queue {
    /// A repeated edge neither replaces its first source identity nor repeats metadata IO.
    pub(super) fn insert(&mut self, source: &Source) {
        if self.entries.contains_key(&source.uri) {
            return;
        }
        let bytes = fs::metadata(&source.path)
            .ok()
            .filter(|metadata| metadata.is_file())
            .map_or(0, |metadata| metadata.len());
        self.restore((bytes, source.clone()));
    }

    /// An unavailable retention slot puts the same hint back without another stat.
    pub(super) fn restore(&mut self, entry: (u64, Source)) {
        let (bytes, source) = entry;
        self.priority.insert((Reverse(bytes), source.uri.clone()));
        self.entries.insert(source.uri.clone(), (bytes, source));
    }

    /// Popping work removes its URI index as well as its priority index.
    pub(super) fn pop_first(&mut self) -> Option<(u64, Source)> {
        let (_, uri) = self.priority.pop_first()?;
        self.entries.remove(&uri)
    }

    /// Canonical consumption cannot leave stale speculative priority entries.
    pub(super) fn remove(&mut self, uri: &str) -> Option<(u64, Source)> {
        let entry = self.entries.remove(uri)?;
        self.priority.remove(&(Reverse(entry.0), uri.to_owned()));
        Some(entry)
    }

    /// Existing frontier limits count candidates, independent of their file lengths.
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[cfg(test)]
    pub(super) fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    #[cfg(test)]
    pub(super) fn contains_key(&self, uri: &str) -> bool {
        self.entries.contains_key(uri)
    }

    #[cfg(test)]
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.priority.clear();
    }
}

#[cfg(test)]
impl FromIterator<Source> for Queue {
    fn from_iter<T: IntoIterator<Item = Source>>(sources: T) -> Self {
        let mut queue = Self::default();
        for source in sources {
            queue.insert(&source);
        }
        queue
    }
}

#[cfg(test)]
mod tests;
