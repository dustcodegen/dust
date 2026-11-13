use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering::Relaxed},
    },
    thread,
};

use dust_parser_dart::ParseResult;
use dust_text::FileId;

use super::{
    ParsePolicy, ResolvedTargets, Source, SourceIndex, SourceInput, prepare::PreparedParse,
};

/// Root facts are keyed by complete source identity and moved once into the loader.
pub(in crate::namespace_sources) struct PreparedRoots {
    /// One extraction policy shared by root and foreign preparation.
    pub(super) policy: Arc<ParsePolicy>,
    /// Unconsumed root facts; no root AST is cloned during canonical loading.
    facts: BTreeMap<String, PreparedParse>,
}

impl PreparedRoots {
    /// Creates a policy even when preparation is unavailable or deliberately unused.
    #[cfg(test)]
    pub(super) fn empty(
        roots: &[Source],
        annotations: &[&'static str],
        snapshots: &BTreeMap<PathBuf, Arc<str>>,
    ) -> Self {
        Self {
            policy: Arc::new(ParsePolicy::new(roots, annotations, snapshots)),
            facts: BTreeMap::new(),
        }
    }

    /// Enumerates actual cached identities, never physical paths alone.
    pub(super) fn uris(&self) -> impl Iterator<Item = &String> {
        self.facts.keys()
    }

    /// Reports whether one canonical read has independently prepared root facts.
    pub(super) fn contains(&self, uri: &str) -> bool {
        self.facts.contains_key(uri)
    }

    /// Consumes facts once, retaining live bytes, flags, options and ownership checks.
    pub(super) fn take(
        &mut self,
        source: &Source,
        input: Option<&SourceInput>,
        id: FileId,
    ) -> Option<(ParseResult, ResolvedTargets)> {
        let facts = self.facts.remove(&source.uri)?;
        let input = input?;
        if input.owner.is_some() {
            return None;
        }
        facts.rebind(source, input, id)
    }
}

/// Parses only discovery snapshots while the coordinator performs output planning.
pub(in crate::namespace_sources) fn prepare_roots<T>(
    index: &SourceIndex,
    roots: &[Source],
    workers: usize,
    annotations: &[&'static str],
    snapshots: &BTreeMap<PathBuf, Arc<str>>,
    cache: Option<Arc<crate::namespace_sources::parser_cache::Cache>>,
    plan: impl FnOnce() -> T,
) -> (T, PreparedRoots) {
    let mut policy = ParsePolicy::new(roots, annotations, snapshots);
    policy.cache = cache;
    let mut prepared = PreparedRoots {
        policy: Arc::new(policy),
        facts: BTreeMap::new(),
    };
    let policy = &prepared.policy;
    let next = AtomicUsize::new(0);
    let prepare = || {
        let mut facts = Vec::new();
        loop {
            let order = next.fetch_add(1, Relaxed);
            let Some(source) = roots.get(order) else {
                return facts;
            };
            if let Some(input) = policy.preloaded_input(source) {
                facts.push((
                    order,
                    source.uri.clone(),
                    PreparedParse::new(index, source, input, policy),
                ));
            }
        }
    };
    let (planned, mut facts) = thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..workers.max(1).min(roots.len()) {
            // Root extraction has the same recursive syntax budget as foreign workers.
            if let Ok(handle) = thread::Builder::new()
                .stack_size(8 * 1024 * 1024)
                .spawn_scoped(scope, prepare)
            {
                handles.push(handle);
            }
        }
        // Any successful worker drains all unclaimed roots, including after failed spawns.
        // If none start, canonical loading retains the existing synchronous fallback.
        let planned = plan();
        let mut facts = Vec::new();
        for handle in handles {
            facts.extend(handle.join().expect("root parser worker must not panic"));
        }
        (planned, facts)
    });
    facts.sort_unstable_by_key(|(order, _, _)| *order);
    prepared.facts = facts
        .into_iter()
        .map(|(_, uri, facts)| (uri, facts))
        .collect();
    (planned, prepared)
}

/// Root caches preserve canonical source validation and full URI identity.
#[cfg(test)]
mod tests;
