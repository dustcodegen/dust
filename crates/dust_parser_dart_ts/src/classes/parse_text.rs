use dust_parser_dart::ParameterKind;
use dust_text::SourceText;
use tree_sitter::Node;

use crate::syntax::{direct_named_child, node_text};

/// Determines whether an optional parameter group is named or positional.
pub(super) fn optional_parameter_kind(node: Node<'_>) -> ParameterKind {
    let mut cursor = node.walk();
    if node.children(&mut cursor).any(|child| child.kind() == "{") {
        ParameterKind::Named
    } else {
        ParameterKind::Positional
    }
}

/// Finds the identifier node that names a formal parameter.
pub(crate) fn parameter_name_node<'tree>(node: Node<'tree>) -> Option<Node<'tree>> {
    if let Some(name) = node.child_by_field_name("name") {
        return Some(name);
    }

    if matches!(node.kind(), "default_formal_parameter") {
        return direct_named_child(node, "formal_parameter").and_then(parameter_name_node);
    }

    for wrapper in [
        "formal_parameter",
        "constructor_param",
        "super_formal_parameter",
    ] {
        if let Some(child) = direct_named_child(node, wrapper) {
            if let Some(name) = parameter_name_node(child) {
                return Some(name);
            }
        }
    }

    direct_named_child(node, "identifier")
}

/// Returns whether a parameter is a Dart field-formal or super-formal parameter.
pub(super) fn is_field_formal_parameter(node: Node<'_>) -> bool {
    direct_named_child(node, "constructor_param").is_some()
        || direct_named_child(node, "super_formal_parameter").is_some()
        || direct_named_child(node, "formal_parameter").is_some_and(is_field_formal_parameter)
}

/// Returns whether a parameter has the explicit Dart `required` modifier.
pub(super) fn is_required_parameter(node: Node<'_>, source: &SourceText) -> bool {
    // The legacy grammar represents a field/super modifier as a type identifier.
    let field_formal = direct_named_child(node, "constructor_param")
        .or_else(|| direct_named_child(node, "super_formal_parameter"));
    if field_formal
        .and_then(|formal| direct_named_child(formal, "type_identifier"))
        .is_some_and(|identifier| node_text(identifier, source) == "required")
    {
        return true;
    }
    let Some(parent) = node.parent() else {
        return false;
    };
    let mut cursor = parent.walk();
    let mut required_since_separator = false;
    for child in parent.children(&mut cursor) {
        if same_node(child, node) {
            return required_since_separator;
        }
        if matches!(child.kind(), "," | "{" | "[") {
            required_since_separator = false;
            continue;
        }
        if child.kind() == "required" {
            required_since_separator = true;
        }
    }
    false
}

/// Extracts a parameter default-value expression from the surrounding syntax.
pub(super) fn default_value_source(node: Node<'_>, source: &SourceText) -> Option<String> {
    let mut candidate = Some(node);
    while let Some(current) = candidate {
        if matches!(
            current.kind(),
            "formal_parameter_list" | "optional_formal_parameters"
        ) {
            break;
        }
        if current.kind() == "default_formal_parameter" {
            let mut value_start = None;
            let mut cursor = current.walk();
            for child in current.children(&mut cursor) {
                if matches!(child.kind(), "=" | ":") {
                    value_start = Some(child.end_byte());
                }
            }
            return default_value_between(source, value_start?, current.end_byte());
        }
        candidate = current.parent();
    }

    let parent = node.parent()?;

    let mut found_parameter = false;
    let mut value_start = None;
    let mut cursor = parent.walk();
    for child in parent.children(&mut cursor) {
        if same_node(child, node) {
            found_parameter = true;
            continue;
        }
        if !found_parameter {
            continue;
        }

        match child.kind() {
            "=" | ":" => value_start = Some(child.end_byte()),
            "," | "}" | "]" => {
                return default_value_between(source, value_start?, child.start_byte());
            }
            _ => {}
        }
    }

    default_value_between(source, value_start?, parent.end_byte())
}

/// Returns trimmed default-value source between two byte offsets.
fn default_value_between(source: &SourceText, start: usize, end: usize) -> Option<String> {
    let value = source.as_str().get(start..end)?.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

/// Compares tree-sitter nodes by stable kind and byte range.
fn same_node(left: Node<'_>, right: Node<'_>) -> bool {
    left.kind() == right.kind()
        && left.start_byte() == right.start_byte()
        && left.end_byte() == right.end_byte()
}
