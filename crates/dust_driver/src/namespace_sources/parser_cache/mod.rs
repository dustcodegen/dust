use std::{
    collections::BTreeMap,
    ops::Range,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering::Relaxed},
};

use dust_parser_dart::{ParseOptions, ParseResult, ParsedDartFileSurface, SourceKind};

use super::{NamespaceSources, index::Source};

/// Compact binary framing, codec and exact option encoding.
mod format;
/// Checked binary indexing without eagerly decoding parser surfaces.
mod frame;
/// Regular-file loading and verified header compatibility.
mod read;
/// One atomic borrowed snapshot write under ordinary Dust cache storage.
mod write;

pub(crate) use write::save;

/// Bumped when the optional parser surface wire contract changes.
const SCHEMA: u32 = 2;

/// Numeric offsets borrow owned bytes only during a lookup, never across moves.
#[derive(Debug)]
struct Record {
    /// Exact parse options, before interpreting a cached surface.
    options: format::Options,
    /// Full-root versus namespace projection mode.
    full: bool,
    /// Original UTF-8 source, checked against current source bytes.
    text: Range<usize>,
    /// Typed surface remains undecoded until a worker requests this URI.
    library: Range<usize>,
}

/// One command's optional parser snapshot; corruption is always a transparent miss.
#[derive(Debug, Default)]
pub(crate) struct Cache {
    /// Owned backing storage for every checked source and AST byte range.
    contents: Vec<u8>,
    /// One unambiguous entry per exact canonical source URI.
    records: BTreeMap<String, Record>,
    /// Only a persistable successful native parse justifies snapshot replacement.
    changed: AtomicBool,
}

impl Cache {
    /// Loads only compatible, checksum-verified regular snapshots.
    pub(crate) fn load(root: &Path, annotations: &[&str]) -> Self {
        read::load(&path(root), annotations).unwrap_or_default()
    }

    /// Uncacheable misses do not justify rewriting an otherwise unchanged snapshot.
    pub(crate) fn needs_save(&self) -> bool {
        self.changed.load(Relaxed)
    }

    /// Caller has completed a native parse with no diagnostics or generated ownership.
    pub(super) fn note_parsed(&self) {
        self.changed.store(true, Relaxed);
    }

    /// Exact current source/options checks precede allocation; lookup never changes state.
    pub(super) fn lookup(
        &self,
        source: &Source,
        text: &str,
        options: ParseOptions,
        full_root: bool,
    ) -> Option<ParseResult> {
        let record = self.records.get(&source.uri)?;
        if record.options != format::options(options) || record.full != full_root {
            return None;
        }
        if self.raw(&record.text)? != text.as_bytes() {
            return None;
        }
        let library: ParsedDartFileSurface = format::decode(self.raw(&record.library)?)?;
        // Native program spans may omit trivia; they must still fit the exact source.
        text.get(library.span.start().to_usize()..library.span.end().to_usize())?;
        Some(ParseResult {
            library,
            diagnostics: Vec::new(),
            namespace_diagnostics: Vec::new(),
            options,
        })
    }

    /// Numeric offsets are bounds checked before constructing a borrowed byte view.
    fn raw(&self, range: &Range<usize>) -> Option<&[u8]> {
        self.contents.get(range.clone())
    }
}

/// Storage stays inside the directory removed by `dust clean`.
fn path(root: &Path) -> PathBuf {
    root.join(".dart_tool/dust/parser_facts_v2.bin")
}

#[cfg(test)]
mod tests;
