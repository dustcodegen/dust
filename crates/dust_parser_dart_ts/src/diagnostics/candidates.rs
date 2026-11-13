//! Candidate discovery stays native; existing Rust helpers decide the meaning.

use std::{cmp::Reverse, collections::BTreeSet, sync::OnceLock};

use dust_parser_dart::{DartLanguageFeature, DartLanguageVersion};
use dust_text::SourceText;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};

/// Every syntax feature/unsupported form inspected by the shared node classifier.
const CANDIDATES: &str = r#"
[
  (record_type)
  (record_literal)
  (pattern_variable_declaration)
  (pattern_assignment)
  (switch_expression)
  (variable_pattern)
  (list_pattern)
  (map_pattern)
  (record_pattern)
  (object_pattern)
  (cast_pattern)
  (null_check_pattern)
  (null_assert_pattern)
  (dot_shorthand)
  (constructor_signature)
  (constant_constructor_signature)
  (factory_constructor_signature)
  (redirecting_factory_constructor_signature)
  (base)
  (interface)
  (sealed)
  (mixin)
] @candidate
(if_statement "case" @candidate)
(if_element "case" @candidate)
(class_definition "final" @candidate)
(list_literal "?" @candidate)
(set_or_map_literal "?" @candidate)
(pair "?" @candidate)
(if_element "?" @candidate)
(for_element "?" @candidate)
"#;

/// Generation limitations remain errors even after the language enables syntax.
const GENERATION_CANDIDATES: &str = r#"
[
  (dot_shorthand)
  (constructor_signature "new")
  (constant_constructor_signature "new")
  (factory_constructor_signature)
  (redirecting_factory_constructor_signature)
] @candidate
"#;

/// Dart 3.0 through 3.7 still need these five collection-element gates.
const NULL_AWARE_COLLECTION_CANDIDATES: &str = r#"
(list_literal "?" @candidate)
(set_or_map_literal "?" @candidate)
(pair "?" @candidate)
(if_element "?" @candidate)
(for_element "?" @candidate)
"#;

/// A grammar/query mismatch falls back to the original full traversal.
fn query(version: Option<DartLanguageVersion>) -> Option<&'static Query> {
    static FULL: OnceLock<Option<Query>> = OnceLock::new();
    static GENERATION: OnceLock<Option<Query>> = OnceLock::new();
    static NULL_AWARE_COLLECTIONS: OnceLock<Option<Query>> = OnceLock::new();
    let Some(version) = version.filter(|version| {
        [
            DartLanguageFeature::Records,
            DartLanguageFeature::Patterns,
            DartLanguageFeature::ClassModifiers,
        ]
        .into_iter()
        .all(|feature| version.supports(feature))
    }) else {
        return FULL
            .get_or_init(|| Query::new(&tree_sitter_dart::LANGUAGE.into(), CANDIDATES).ok())
            .as_ref();
    };
    if version.supports(DartLanguageFeature::NullAwareCollectionElements) {
        GENERATION
            .get_or_init(|| {
                Query::new(&tree_sitter_dart::LANGUAGE.into(), GENERATION_CANDIDATES).ok()
            })
            .as_ref()
    } else {
        NULL_AWARE_COLLECTIONS
            .get_or_init(|| {
                Query::new(
                    &tree_sitter_dart::LANGUAGE.into(),
                    &format!("{GENERATION_CANDIDATES}{NULL_AWARE_COLLECTION_CANDIDATES}"),
                )
                .ok()
            })
            .as_ref()
    }
}

/// Restores exact source preorder, including equal-span nested candidate nodes.
pub(super) fn collect<'tree>(
    tree: &'tree Tree,
    source: &SourceText,
    version: Option<DartLanguageVersion>,
) -> Option<Vec<Node<'tree>>> {
    let query = query(version)?;
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, tree.root_node(), source.as_str().as_bytes());
    let mut nodes = Vec::new();
    let mut seen = BTreeSet::new();
    while let Some(matched) = matches.next() {
        for capture in matched.captures {
            let node = capture.node;
            if seen.insert(node.id()) {
                let depth = std::iter::successors(node.parent(), |parent| parent.parent()).count();
                nodes.push((node, depth));
            }
        }
    }
    drop(matches);
    if cursor.did_exceed_match_limit() {
        return None;
    }
    nodes.sort_unstable_by_key(|(node, depth)| {
        (node.start_byte(), Reverse(node.end_byte()), *depth)
    });
    Some(nodes.into_iter().map(|(node, _)| node).collect())
}

#[test]
fn query_supports_the_active_grammar() {
    let legacy = super::tests::uses_legacy_grammar();
    for version in [
        None,
        Some(DartLanguageVersion::DART_3_0),
        Some(DartLanguageVersion::DART_3_8),
    ] {
        if legacy {
            assert!(query(version).is_none(), "{version:?}");
            assert_legacy_fallback_parity(version);
        } else {
            assert!(query(version).is_some(), "{version:?}");
        }
    }
}

/// The separately linked legacy grammar lacks the production query's node kinds.
#[cfg(test)]
fn assert_legacy_fallback_parity(version: Option<DartLanguageVersion>) {
    for (text, valid) in [
        ("(int, String) f() => (1, 'one');", true),
        (
            "void f(Object value) { if(value case [var item]) {} }",
            true,
        ),
        ("class Broken { void f( {}", false),
    ] {
        let source = SourceText::new(dust_text::FileId::new(146), text);
        let tree = crate::PARSER
            .with(|parser| parser.borrow_mut().parse(text, None))
            .unwrap();
        assert_eq!(!tree.root_node().has_error(), valid, "{text}");
        assert!(collect(&tree, &source, version).is_none());
        let (actual, actual_occurrences) =
            super::tests::assert_full_walk_parity(&tree, &source, version);
        if valid {
            assert!(!actual_occurrences.is_empty(), "{text}");
        } else {
            assert!(!actual.diagnostics.is_empty(), "{text}");
            assert!(!actual.namespace_diagnostics.is_empty(), "{text}");
        }
    }
}
