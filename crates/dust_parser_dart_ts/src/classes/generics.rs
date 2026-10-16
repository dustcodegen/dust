use dust_parser_dart::{ParsedTypeParamSurface, ParsedTypeSurface};
use dust_text::SourceText;
use tree_sitter::Node;

use crate::{
    syntax::{direct_named_child, find_first_descendant, node_text, text_range},
    types::{extract_type_before, extract_type_node},
};

/// Extracts only the owning class's type parameters, never member binders.
pub(super) fn extract_class_type_parameters(
    class: Node<'_>,
    source: &SourceText,
) -> Vec<ParsedTypeParamSurface> {
    let Some(parameters) = class
        .child_by_field_name("type_parameters")
        .or_else(|| direct_named_child(class, "type_parameters"))
    else {
        return Vec::new();
    };
    let mut cursor = parameters.walk();
    parameters
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "type_parameter")
        .filter_map(|parameter| extract_type_parameter(parameter, source))
        .collect()
}

/// Extracts one parameter name and its independent original bound surface.
fn extract_type_parameter(
    parameter: Node<'_>,
    source: &SourceText,
) -> Option<ParsedTypeParamSurface> {
    let name = direct_named_child(parameter, "type_identifier")
        .or_else(|| direct_named_child(parameter, "identifier"))?;
    let bound = direct_named_child(parameter, "type_bound")
        .and_then(|bound| extract_type_before(bound, bound.end_byte(), source));
    Some(ParsedTypeParamSurface {
        name: node_text(name, source),
        name_span: text_range(name),
        bound,
        span: text_range(parameter),
    })
}

/// Extracts written mixins without treating `extends` or `implements` as mixins.
pub(super) fn extract_class_mixins(class: Node<'_>, source: &SourceText) -> Vec<ParsedTypeSurface> {
    let Some(mixins) = class
        .child_by_field_name("superclass")
        .and_then(|superclass| find_first_descendant(superclass, "mixins"))
        .or_else(|| direct_named_child(class, "mixins"))
    else {
        return Vec::new();
    };
    let mut result = Vec::new();
    let mut first = None;
    let mut end = 0;
    let mut cursor = mixins.walk();
    for child in mixins.children(&mut cursor) {
        if child.kind() == "," {
            push_mixin(&mut result, first.take(), end, source);
        } else if child.is_named() && !matches!(child.kind(), "comment" | "documentation_comment") {
            first.get_or_insert(child);
            end = child.end_byte();
        }
    }
    push_mixin(&mut result, first, end, source);
    result
}

/// Completes one direct-child comma-delimited mixin type.
fn push_mixin(
    result: &mut Vec<ParsedTypeSurface>,
    first: Option<Node<'_>>,
    end: usize,
    source: &SourceText,
) {
    if let Some(parsed) = first.and_then(|first| extract_type_node(first, end, source)) {
        result.push(parsed);
    }
}
