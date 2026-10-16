use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_text::SourceText;
use tree_sitter::{Node, Tree};

use crate::language_gates::{FeatureOccurrence, record_syntax_occurrence};
use crate::syntax::{direct_named_child, node_text, text_range};

/// Extracts syntax diagnostics reported by tree-sitter.
pub(crate) fn extract_diagnostics(
    tree: &Tree,
    source: &SourceText,
    occurrences: &mut Vec<FeatureOccurrence>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    collect_error_diagnostics(tree.root_node(), source, &mut diagnostics, occurrences);
    if let Some(first) = diagnostics
        .iter_mut()
        .find(|diagnostic| diagnostic.message == "tree-sitter reported a Dart syntax error")
    {
        first.notes.push(
            "Check this file with `dart analyze`. Valid Dart may use syntax not supported by Dust's bundled parser.".to_owned(),
        );
        first.notes.push(
            "Named concise `new` constructors are not supported yet. Use an in-body constructor with the class name.".to_owned(),
        );
    }
    diagnostics
}

/// Records parse errors and grammar forms whose generation is still unsupported.
fn collect_error_diagnostics(
    node: Node<'_>,
    source: &SourceText,
    diagnostics: &mut Vec<Diagnostic>,
    occurrences: &mut Vec<FeatureOccurrence>,
) {
    record_syntax_occurrence(node, source, occurrences);
    if let Some(feature) = unsupported_feature(node, source) {
        let guidance = if feature == "dot shorthands" {
            "Use an explicit type name instead of a dot shorthand."
        } else {
            "Use traditional in-body constructors with the class name and explicit fields."
        };
        diagnostics.push(
            Diagnostic::error(format!("Dust does not support {feature} yet"))
                .with_label(SourceLabel::new(source.file_id(), text_range(node), "unsupported code generation"))
                .with_note("Check this file with `dart analyze`. Dust's bundled parser accepts this syntax, but code generation is not implemented yet.")
                .with_note(guidance),
        );
        return;
    } else if node.is_error() || node.is_missing() {
        diagnostics.push(
            Diagnostic::error("tree-sitter reported a Dart syntax error").with_label(
                SourceLabel::new(source.file_id(), text_range(node), "invalid syntax"),
            ),
        );
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_error_diagnostics(child, source, diagnostics, occurrences);
    }
}

/// Keeps newly parseable syntax gated until its generated consumers are verified.
fn unsupported_feature(node: Node<'_>, source: &SourceText) -> Option<&'static str> {
    match node.kind() {
        "dot_shorthand" if !is_generic_receiver_member(node, source) => Some("dot shorthands"),
        "constructor_signature" | "constant_constructor_signature"
            if is_concise_constructor(node) =>
        {
            Some("concise constructors")
        }
        "factory_constructor_signature" | "redirecting_factory_constructor_signature"
            if is_concise_factory(node, source) =>
        {
            Some("concise constructors")
        }
        _ => None,
    }
}

/// The grammar can parse `Type<T>.method` as nested relational expressions.
pub(crate) fn is_generic_receiver_member(node: Node<'_>, source: &SourceText) -> bool {
    let Some(close) = node.prev_named_sibling() else {
        return false;
    };
    let Some(receiver) = close
        .prev_named_sibling()
        .filter(|_| close.kind() == "relational_operator" && node_text(close, source) == ">")
    else {
        return false;
    };
    let mut cursor = receiver.walk();
    receiver.kind() == "relational_expression"
        && receiver
            .named_children(&mut cursor)
            .any(|child| child.kind() == "relational_operator" && node_text(child, source) == "<")
}

/// Only a leading `new` is concise; a traditional `Type.new` is not.
fn is_concise_constructor(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| matches!(child.kind(), "new" | "identifier"))
        .is_some_and(|child| child.kind() == "new")
}

/// Concise factories omit the containing class name from the signature.
fn is_concise_factory(node: Node<'_>, source: &SourceText) -> bool {
    std::iter::successors(node.parent(), |parent| parent.parent())
        .find(|parent| {
            matches!(
                parent.kind(),
                "class_definition" | "class_declaration" | "enum_declaration"
            )
        })
        .is_some_and(|parent| {
            let class_name = parent
                .child_by_field_name("name")
                .or_else(|| direct_named_child(parent, "identifier"));
            let constructor_name = direct_named_child(node, "identifier");
            match (class_name, constructor_name) {
                (Some(class), Some(constructor)) => {
                    node_text(class, source) != node_text(constructor, source)
                }
                _ => true,
            }
        })
}
