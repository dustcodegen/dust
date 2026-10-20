//! Modern Rust binding for Dust's pinned, unchanged listed-upstream Dart grammar.
//!
//! Dust uses this binding after extraction parity checks. Grammar acceptance
//! alone does not establish generated-code support for new syntax.

use tree_sitter_language::LanguageFn;

extern "C" {
    /// Returns the static language descriptor from the pinned generated parser.
    fn tree_sitter_dart() -> *const ();
}

/// The pinned Dart grammar, compatible with Dust's Tree-sitter 0.25 runtime.
// SAFETY: The vendored C export has Tree-sitter's language-function signature
// and returns the immutable, static ABI-15 descriptor tested by this crate.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_dart) };

/// Node definitions from the exact same upstream revision as the compiled parser.
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");
