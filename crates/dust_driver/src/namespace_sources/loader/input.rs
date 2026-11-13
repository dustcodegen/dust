use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::Arc,
};

use dust_parser_dart::{ParseOptions, ParseResult, parse_file_with_backend};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_text::{FileId, SourceText};

use super::Source;

/// Generation roots keep complete payloads; external libraries need interfaces.
#[derive(Default)]
pub(super) struct ParsePolicy {
    /// Every discovered handwritten workspace source is a generation input.
    roots: HashSet<String>,
    /// External metadata with these structured names needs plugin analysis.
    annotations: Vec<&'static str>,
    /// Exact handwritten bytes retained by the initial discovery read.
    snapshots: HashMap<PathBuf, Arc<str>>,
    /// Source-exact parser facts shared by this command's preparation workers.
    pub(super) cache: Option<Arc<crate::namespace_sources::parser_cache::Cache>>,
}

impl ParsePolicy {
    /// Keeps one immutable extraction policy for local and prepared parses.
    pub(super) fn new(
        roots: &[Source],
        annotations: &[&'static str],
        snapshots: &BTreeMap<PathBuf, Arc<str>>,
    ) -> Self {
        Self {
            roots: roots.iter().map(|source| source.uri.clone()).collect(),
            annotations: annotations.to_vec(),
            snapshots: snapshots
                .iter()
                .map(|(path, text)| (path.clone(), Arc::clone(text)))
                .collect(),
            cache: None,
        }
    }

    /// Speculation may reuse discovery bytes; the coordinator still rereads them.
    pub(super) fn preloaded_input(&self, source: &Source) -> Option<SourceInput> {
        Some(SourceInput {
            text: Arc::clone(self.snapshots.get(&source.path)?),
            owner: None,
            generated_library: false,
        })
    }

    /// Counts the known workspace sources whose facts will be retained after loading.
    pub(super) fn root_count(&self) -> usize {
        self.roots.len()
    }
}

/// The source text and generated-file classification used by one parse.
#[derive(PartialEq, Eq)]
pub(super) struct SourceInput {
    /// Dart text, empty for generated placeholders.
    pub(super) text: Arc<str>,
    /// Declaring root for a generated primary part.
    pub(super) owner: Option<String>,
    /// Whether a generated library was replaced by a placeholder.
    pub(super) generated_library: bool,
}

impl SourceInput {
    /// Reads handwritten text or prepares an empty generated placeholder.
    pub(super) fn read(
        source: &Source,
        owner: Option<String>,
        potential: &BTreeSet<PathBuf>,
    ) -> Result<Self, String> {
        Self::read_with_snapshot(source, owner, potential, None)
    }

    /// Shares discovery storage only after a complete successful live read agrees.
    pub(super) fn read_with_snapshot(
        source: &Source,
        owner: Option<String>,
        potential: &BTreeSet<PathBuf>,
        snapshot: Option<&Arc<str>>,
    ) -> Result<Self, String> {
        let mut generated_library = false;
        let text = if owner.is_some() {
            String::new()
        } else {
            let text = match fs::read_to_string(&source.path) {
                Ok(text) => text,
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound
                        && potential.contains(&source.path) =>
                {
                    generated_library = true;
                    String::new()
                }
                Err(error) => {
                    return Err(format!(
                        "cannot read namespace source `{}`: {error}",
                        source.path.display()
                    ));
                }
            };
            generated_library |= crate::generated_source::is_dust_generated_source(&text);
            if generated_library {
                String::new()
            } else {
                text
            }
        };
        let text = match snapshot {
            Some(snapshot)
                if owner.is_none() && !generated_library && snapshot.as_ref() == text.as_str() =>
            {
                Arc::clone(snapshot)
            }
            _ => text.into(),
        };
        Ok(Self {
            text,
            owner,
            generated_library,
        })
    }

    /// Extracts parser facts independently of directive resolution and the graph.
    pub(super) fn parse(&self, source: &Source, id: FileId, policy: &ParsePolicy) -> ParseResult {
        let options = Self::options(source);
        let full_root = policy.roots.contains(&source.uri);
        if self.owner.is_none()
            && !self.generated_library
            && let Some(cached) = policy
                .cache
                .as_ref()
                .and_then(|cache| cache.lookup(source, &self.text, options, full_root))
        {
            return cached;
        }
        let backend = TreeSitterDartBackend::new();
        let text = SourceText::new(id, Arc::clone(&self.text));
        let parsed = if full_root {
            parse_file_with_backend(&backend, &text, options)
        } else {
            backend.parse_namespace_file(&text, options, &policy.annotations)
        };
        if self.owner.is_none()
            && !self.generated_library
            && parsed.diagnostics.is_empty()
            && parsed.namespace_diagnostics.is_empty()
            && let Some(cache) = &policy.cache
        {
            cache.note_parsed();
        }
        parsed
    }

    /// Captures the parser settings independently of source identity.
    pub(super) fn options(source: &Source) -> ParseOptions {
        ParseOptions {
            language_version: source.language_version,
            ..Default::default()
        }
    }
}

/// The same input policy governs synchronous and prepared source extraction.
#[cfg(test)]
mod tests;
