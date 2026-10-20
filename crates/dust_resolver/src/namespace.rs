use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::SymbolId;
use dust_parser_dart::{ParsedDartFileSurface, ParsedUriLiteral};
use dust_text::{FileId, TextRange};

use crate::SymbolCatalog;

/// Shared conditional ownership and concrete failure precedence.
mod agreement;
/// Parser facts become immutable graph inputs with caller-resolved identities.
mod input;
/// Source-position-aware lexical shadows and constant identity.
mod lexical;
/// Terminating export computation, visibility, imports and part ownership.
mod lookup;
/// Namespace declarations, alternatives and source-attached input failures.
mod model;
/// Declaration-origin ownership and transparent alias binding.
mod owner;

use agreement::agree;
use model::*;

/// Why a written annotation name cannot be safely bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamespaceFailureKind {
    /// An import prefix does not exist in this library.
    UndeclaredPrefix,
    /// The name is absent or hidden in a known namespace.
    Hidden,
    /// Distinct visible declarations or canonical owners compete.
    Ambiguous,
    /// Relevant conditional alternatives have different ownership.
    Conditional,
    /// Metadata refers to a deferred namespace.
    Deferred,
    /// Required source, URI or ownership facts are unavailable.
    MissingInput,
}

/// Ownership is not conflated with visibility failure or unknown provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceBinding {
    /// One visible declaration belongs to this registered semantic owner.
    Owned(SymbolId),
    /// A known declaration is foreign to the registered annotation owners.
    Foreign,
    /// Provenance is genuinely unknown; only unique legacy lookup may apply.
    Unknown,
    /// No short-name fallback may bypass this failure.
    Rejected {
        /// The distinct failure category.
        kind: NamespaceFailureKind,
        /// Definition/import labels, before adding the annotation use label.
        diagnostic: Diagnostic,
    },
}

/// An immutable source graph after insertion; no filesystem or host target guesses.
#[derive(Debug, Default)]
pub struct NamespaceGraph {
    /// Source units keyed by already resolved library/part identity.
    units: BTreeMap<String, Unit>,
    /// Name-specific export results are shared safely across resolver workers.
    exports: Mutex<BTreeMap<String, Arc<ExportWorlds>>>,
    /// Library/part import namespaces required by queried lexical initializers.
    lexical_imports: Mutex<BTreeSet<String>>,
}

/// A library-scoped view of the same graph and catalog ownership policy.
#[derive(Debug, Clone, Copy)]
pub struct NamespaceScope<'a> {
    /// The source graph, never a reconstructed package-name heuristic.
    graph: &'a NamespaceGraph,
    /// Root or part URI supplied by the source loader.
    uri: &'a str,
}

impl NamespaceGraph {
    /// Creates a graph with no supplied sources; missing sources stay explicit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds parser facts once, resolving decoded URI values through the loader.
    ///
    /// Normalization is delegated, not guessed. Callback failures are retained
    /// on their edges so unrelated prefixes/hidden names are not blanket errors.
    pub fn insert_library(
        &mut self,
        uri: impl Into<String>,
        file_id: FileId,
        parsed: &ParsedDartFileSurface,
        mut resolve_uri: impl FnMut(&str, &str) -> Result<String, String>,
    ) -> Result<(), Diagnostic> {
        let uri = uri.into();
        if self.units.contains_key(&uri) || self.units.values().any(|unit| unit.file_id == file_id)
        {
            return Err(Diagnostic::error(format!(
                "duplicate namespace input identity `{uri}` or file id {}",
                file_id.raw()
            )));
        }
        let unit = input::unit(&uri, file_id, parsed, &mut resolve_uri);
        self.units.insert(uri, unit);
        self.exports
            .get_mut()
            .expect("namespace cache is not poisoned")
            .clear();
        Ok(())
    }

    /// Creates a scoped view without pretending missing input is a foreign library.
    pub fn scope<'a>(&'a self, uri: &'a str) -> NamespaceScope<'a> {
        NamespaceScope { graph: self, uri }
    }

    /// Actual initializer namespaces needed before a production binding is final.
    pub fn required_lexical_imports(&self) -> BTreeSet<String> {
        self.lexical_imports
            .lock()
            .expect("lexical import requests are not poisoned")
            .clone()
    }
}

impl NamespaceScope<'_> {
    /// Binds a normalized dotted name using actual declarations before role choice.
    pub fn bind(&self, name: &str, catalog: &SymbolCatalog) -> NamespaceBinding {
        self.graph
            .bind_name(self.uri, name, catalog, &mut BTreeSet::new())
    }

    /// Binds metadata at its actual source use, before consulting imported names.
    pub fn bind_at(
        &self,
        name: &str,
        catalog: &SymbolCatalog,
        span: dust_ir::SpanIr,
    ) -> NamespaceBinding {
        self.graph
            .bind_at(self.uri, name, catalog, span, &mut BTreeSet::new())
    }
}

/// Builds a typed rejection without erasing source labels.
fn rejected(
    kind: NamespaceFailureKind,
    message: impl Into<String>,
    labels: Vec<SourceLabel>,
) -> NamespaceBinding {
    let mut diagnostic = Diagnostic::error(message);
    diagnostic.labels = labels;
    NamespaceBinding::Rejected { kind, diagnostic }
}

/// Decodes one exact literal before the caller resolves its library identity.
fn target(
    uri: &str,
    file_id: FileId,
    literal: Option<&ParsedUriLiteral>,
    span: TextRange,
    resolve: &mut impl FnMut(&str, &str) -> Result<String, String>,
) -> Target {
    literal
        .and_then(ParsedUriLiteral::value)
        .ok_or_else(|| format!("missing or unsupported namespace URI literal in `{uri}`"))
        .and_then(|value| resolve(uri, &value))
        .map_err(|message| Problem::new(file_id, span, message))
}
