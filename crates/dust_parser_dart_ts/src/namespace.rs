//! Namespace extraction falls back whenever complete generation facts are needed.

use dust_parser_dart::DartLanguageVersion;
use dust_text::SourceText;
use std::sync::OnceLock;
use tree_sitter::{Node, TreeCursor};

use crate::{
    annotations::annotation_name,
    classes::parameter_name_node,
    language_gates::has_invalid_declaring_modifier,
    syntax::{direct_child, text_range},
};

/// Existing parameter-derived language diagnostics must keep their original facts.
pub(crate) fn can_project(
    root: Node<'_>,
    source: &SourceText,
    version: DartLanguageVersion,
    full_surface_annotations: &[&str],
) -> bool {
    let legacy_defaults = version >= DartLanguageVersion::DART_3_13 && !has_dot_shorthand_nodes();
    !root.has_error()
        && !needs_full_surface(
            &mut root.walk(),
            source,
            version,
            full_surface_annotations,
            legacy_defaults,
        )
}

/// Grammars without dot nodes cannot use native diagnostics to retain their defaults.
fn has_dot_shorthand_nodes() -> bool {
    static SUPPORTED: OnceLock<bool> = OnceLock::new();
    *SUPPORTED.get_or_init(|| {
        let language: tree_sitter::Language = tree_sitter_dart::LANGUAGE.into();
        language.id_for_node_kind("dot_shorthand", true) != 0
    })
}

/// Inspect structured names at every site, including nested callback parameters.
fn needs_full_surface(
    cursor: &mut TreeCursor<'_>,
    source: &SourceText,
    version: DartLanguageVersion,
    annotations: &[&str],
    legacy_defaults: bool,
) -> bool {
    let node = cursor.node();
    if is_supported_annotation(node, source, annotations) {
        return true;
    }
    if version < DartLanguageVersion::DART_3_12 && is_private_named_parameter(node, source) {
        return true;
    }
    if version >= DartLanguageVersion::DART_3_13
        && requires_parameter_facts(node, source, legacy_defaults)
    {
        return true;
    }
    // Metadata and declaring parameter names cannot occur in implementation
    // bodies or initializer expressions represented as values by the parser.
    if prunes_children(node) {
        return false;
    }
    if cursor.goto_first_child() {
        loop {
            if cursor.node().is_named()
                && needs_full_surface(cursor, source, version, annotations, legacy_defaults)
            {
                return true;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
    false
}

/// Primary constructors and invalid modifiers need complete language-validation facts.
fn requires_parameter_facts(node: Node<'_>, source: &SourceText, legacy_defaults: bool) -> bool {
    match node.kind() {
        "primary_constructor" => true,
        "formal_parameter" | "default_formal_parameter" => {
            has_invalid_declaring_modifier(source, text_range(node))
                || legacy_defaults
                    && node.kind() == "default_formal_parameter"
                    && is_constructor_parameter_group(node.parent())
        }
        "optional_formal_parameters" if legacy_defaults => {
            is_constructor_parameter_group(Some(node))
                && (direct_child(node, "=").is_some() || direct_child(node, ":").is_some())
        }
        _ => false,
    }
}

/// A callback's optional group belongs to its own signature, not an outer constructor.
fn is_constructor_parameter_group(group: Option<Node<'_>>) -> bool {
    group
        .filter(|group| group.kind() == "optional_formal_parameters")
        .and_then(|group| group.parent())
        .filter(|list| list.kind() == "formal_parameter_list")
        .and_then(|list| list.parent())
        .is_some_and(|signature| {
            matches!(
                signature.kind(),
                "constructor_signature"
                    | "constant_constructor_signature"
                    | "factory_constructor_signature"
                    | "redirecting_factory_constructor_signature"
            )
        })
}

/// Unqualified annotation names need no owned strings or argument extraction.
fn is_supported_annotation(node: Node<'_>, source: &SourceText, annotations: &[&str]) -> bool {
    node.kind() == "annotation"
        && node.child_by_field_name("name").is_some_and(|name| {
            if matches!(name.kind(), "qualified" | "scoped_identifier") {
                annotations.contains(&annotation_name(name, source).0.as_str())
            } else {
                source
                    .slice(text_range(name))
                    .is_some_and(|name| annotations.contains(&name))
            }
        })
}

/// Metadata discovery excludes values, implementation bodies and trivia.
fn prunes_children(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_body"
            | "initialized_identifier_list"
            | "static_final_declaration_list"
            | "arguments"
            | "annotation_arguments"
            | "list_literal"
            | "set_or_map_literal"
            | "record_literal"
            | "comment"
            | "documentation_comment"
    )
}

/// Overinclusive fallback is safe; no private parameter diagnostic can be omitted.
fn is_private_named_parameter(node: Node<'_>, source: &SourceText) -> bool {
    matches!(node.kind(), "formal_parameter" | "default_formal_parameter")
        && parameter_name_node(node).is_some_and(|name| {
            source
                .slice(text_range(name))
                .is_some_and(|name| name.starts_with('_'))
        })
        && std::iter::successors(node.parent(), |parent| parent.parent())
            .find(|parent| {
                matches!(
                    parent.kind(),
                    "formal_parameter_list" | "optional_formal_parameters"
                )
            })
            .is_some_and(|group| {
                group.kind() == "optional_formal_parameters" && direct_child(group, "{").is_some()
            })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod preflight_tests;

#[cfg(test)]
mod modern_tests;
