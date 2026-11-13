use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, OnceLock},
};

use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::SymbolId;
use dust_parser_dart::{ParsedDartFileSurface, ParsedDirective, ParsedUriLiteral};
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
/// Owning normalization can precede ordered graph insertion.
mod prepared;
/// Concrete type identities independent of annotation ownership.
mod type_origin;

pub use prepared::PreparedNamespaceLibrary;
pub use type_origin::TypeDeclarationOrigin;

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
    /// URI of each source unit by its diagnostic file identity.
    file_uris: BTreeMap<FileId, String>,
    /// Name-specific export results are shared safely across resolver workers.
    exports: Mutex<BTreeMap<String, Arc<OnceLock<Arc<ExportWorlds>>>>>,
    /// Namespace-only queries remain independent of catalog and lexical use sites.
    lookups: lookup::LookupCache,
    /// Library/part imports required by queried initializers and type aliases.
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
        resolve_uri: impl FnMut(&str, &str) -> Result<String, String>,
    ) -> Result<(), Diagnostic> {
        self.insert_library_with_appended_directive(uri, file_id, parsed, None, resolve_uri)
    }

    /// Adds a library with an optional synthetic directive after all written directives.
    pub fn insert_library_with_appended_directive(
        &mut self,
        uri: impl Into<String>,
        file_id: FileId,
        parsed: &ParsedDartFileSurface,
        appended_directive: Option<&ParsedDirective>,
        resolve_uri: impl FnMut(&str, &str) -> Result<String, String>,
    ) -> Result<(), Diagnostic> {
        let uri = uri.into();
        self.check_input_identity(&uri, file_id)?;
        let prepared = Self::prepare_library(uri, file_id, parsed, appended_directive, resolve_uri);
        self.insert_prepared_library(prepared)
    }

    /// Finds the retained source unit for a diagnostic file identity.
    fn unit_by_file_id(&self, file_id: FileId) -> Option<&Unit> {
        self.file_uris
            .get(&file_id)
            .and_then(|uri| self.units.get(uri))
    }

    /// Creates a scoped view without pretending missing input is a foreign library.
    pub fn scope<'a>(&'a self, uri: &'a str) -> NamespaceScope<'a> {
        NamespaceScope { graph: self, uri }
    }

    /// Clears discovery requests between sequential ownership passes.
    pub fn clear_required_lexical_imports(&mut self) {
        self.lexical_imports
            .get_mut()
            .expect("lexical import requests are not poisoned")
            .clear();
    }

    /// Queried initializer and alias namespaces needed before a binding is final.
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
