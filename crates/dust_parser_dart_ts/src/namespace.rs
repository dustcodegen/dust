//! Namespace extraction falls back whenever complete generation facts are needed.

use dust_parser_dart::DartLanguageVersion;
use dust_text::SourceText;
use tree_sitter::{Node, TreeCursor};

use crate::{
    annotations::annotation_name,
    classes::parameter_name_node,
    syntax::{direct_child, text_range},
};

/// Existing parameter-derived language diagnostics must keep their original facts.
pub(crate) fn can_project(
    root: Node<'_>,
    source: &SourceText,
    version: DartLanguageVersion,
    full_surface_annotations: &[&str],
) -> bool {
    let private_parameters = version < DartLanguageVersion::DART_3_12;
    !root.has_error()
        && version < DartLanguageVersion::DART_3_13
        && !needs_full_surface(
            &mut root.walk(),
            source,
            private_parameters,
            full_surface_annotations,
        )
}

/// Inspect structured names at every site, including nested callback parameters.
fn needs_full_surface(
    cursor: &mut TreeCursor<'_>,
    source: &SourceText,
    check_private_parameters: bool,
    annotations: &[&str],
) -> bool {
    let node = cursor.node();
    if is_supported_annotation(node, source, annotations) {
        return true;
    }
    if check_private_parameters && is_private_named_parameter(node, source) {
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
                && needs_full_surface(cursor, source, check_private_parameters, annotations)
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
