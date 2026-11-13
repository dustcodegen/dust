use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{WorkspaceAnalysis, WorkspaceAnalysisBuilder};
use crate::TypeOriginRequest;

/// Cached type identity uses source bytes rather than transient parser file IDs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(super) struct TypeOriginFact {
    /// Handwritten source path that owns the type reference.
    source_path: String,
    /// Written type name, retaining its import prefix.
    name: String,
    /// Inclusive byte offset of the reference's source span.
    start: u32,
    /// Exclusive byte offset of the reference's source span.
    end: u32,
    /// Canonical URI of the declaration's owning library.
    library_uri: String,
    /// Concrete declaration name without an import prefix.
    declaration_name: String,
}

impl TypeOriginFact {
    /// Matches stable source identity without consulting transient parser file IDs.
    fn matches_reference(&self, source_path: &str, request: &TypeOriginRequest) -> bool {
        self.source_path == source_path
            && self.name == request.name
            && self.start == request.span.range.start().to_u32()
            && self.end == request.span.range.end().to_u32()
    }
}

impl WorkspaceAnalysisBuilder {
    /// Records one canonical declaration candidate for a source type reference.
    ///
    /// Different candidates for the same reference remain recorded so ownership
    /// checks fail closed, including after merging cached library snapshots.
    pub fn add_type_origin(
        &mut self,
        source_path: &str,
        request: &TypeOriginRequest,
        library_uri: &str,
        declaration_name: &str,
    ) {
        self.type_origins.insert(TypeOriginFact {
            source_path: source_path.to_owned(),
            name: request.name.clone(),
            start: request.span.range.start().to_u32(),
            end: request.span.range.end().to_u32(),
            library_uri: library_uri.to_owned(),
            declaration_name: declaration_name.to_owned(),
        });
    }

    /// Returns whether a reference has exactly the requested declaration origin.
    ///
    /// Missing or conflicting origins return `false`. File IDs are deliberately
    /// ignored so a cached proof survives parser ID reassignment.
    pub fn has_type_origin(
        &self,
        source_path: &str,
        request: &TypeOriginRequest,
        library_uri: &str,
        declaration_name: &str,
    ) -> bool {
        has_origin(
            &self.type_origins,
            source_path,
            request,
            library_uri,
            declaration_name,
        )
    }
}

impl WorkspaceAnalysis {
    /// Returns whether a reference has exactly the requested declaration origin.
    ///
    /// Missing or conflicting origins return `false`; transient file IDs do not
    /// participate in source identity.
    pub fn has_type_origin(
        &self,
        source_path: &str,
        request: &TypeOriginRequest,
        library_uri: &str,
        declaration_name: &str,
    ) -> bool {
        has_origin(
            &self.type_origins,
            source_path,
            request,
            library_uri,
            declaration_name,
        )
    }
}

/// Accepts exactly one declaration candidate matching the requested origin.
fn has_origin(
    facts: &BTreeSet<TypeOriginFact>,
    source_path: &str,
    request: &TypeOriginRequest,
    library_uri: &str,
    declaration_name: &str,
) -> bool {
    let mut candidates = facts
        .iter()
        .filter(|fact| fact.matches_reference(source_path, request));
    candidates.next().is_some_and(|fact| {
        fact.library_uri == library_uri && fact.declaration_name == declaration_name
    }) && candidates.next().is_none()
}

/// Appends sorted typed facts while preserving the old hash when no facts exist.
pub(super) fn hash_type_origins(hash: &mut u64, facts: &BTreeSet<TypeOriginFact>) {
    if facts.is_empty() {
        return;
    }
    hash_bytes(hash, b"\xffdust.type_origins.v1\0");
    for fact in facts {
        for field in [&fact.source_path, &fact.name] {
            hash_bytes(hash, field.as_bytes());
            hash_bytes(hash, &[0]);
        }
        hash_bytes(hash, &fact.start.to_le_bytes());
        hash_bytes(hash, &fact.end.to_le_bytes());
        for field in [&fact.library_uri, &fact.declaration_name] {
            hash_bytes(hash, field.as_bytes());
            hash_bytes(hash, &[0]);
        }
    }
}

/// Continues the workspace analysis FNV-1a hash with an exact byte sequence.
fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(1099511628211);
    }
}

#[cfg(test)]
mod tests;
