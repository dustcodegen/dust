use dust_dart_syntax::parse_static_dart_string_literal;
use dust_parser_dart::{
    ParsedAnnotationNumberKind, ParsedAnnotationValue, ParsedAnnotationValueRootKind,
};
use dust_text::{SourceText, TextRange};
use tree_sitter::Node;

use super::extract_annotation_arguments;
use crate::syntax::{direct_named_child, has_descendant_kind, node_text, text_range};

/// Reading list, set, map and constructor argument values out of an annotation.
mod collections;
use self::collections::*;

/// Structured constructor names and arguments.
mod constructors;
use self::constructors::constructor_kind;

/// Converts an annotation argument container into a parser-owned value.
pub(super) fn annotation_value_from_container(
    container: Node<'_>,
    value_source: String,
    value_span: TextRange,
    source: &SourceText,
) -> Option<ParsedAnnotationValue> {
    if is_member_selector_chain(container) && has_descendant_kind(container, "arguments") {
        return Some(ParsedAnnotationValue {
            source: value_source,
            span: value_span,
            kind: constructor_kind([container], source)
                .unwrap_or(ParsedAnnotationValueRootKind::Expression),
        });
    }

    let value_node = annotation_argument_value_node(container, value_span)?;
    Some(annotation_value(
        value_node,
        source,
        value_source,
        value_span,
        is_member_selector_chain(container),
    ))
}

/// Returns the expression node inside one annotation argument container.
fn annotation_argument_value_node(node: Node<'_>, value_span: TextRange) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.is_named() && !child.is_extra() && child.kind() != "label")
        .find(|child| {
            let child_range = text_range(*child);
            child_range.start() >= value_span.start() && child_range.end() <= value_span.end()
        })
}

/// Converts a tree-sitter value node into a parser-owned annotation value.
fn annotation_value(
    node: Node<'_>,
    source: &SourceText,
    value_source: String,
    value_span: TextRange,
    force_member: bool,
) -> ParsedAnnotationValue {
    let kind = if force_member {
        ParsedAnnotationValueRootKind::Member(value_source.clone())
    } else {
        match node.kind() {
            "null_literal" => ParsedAnnotationValueRootKind::Null,
            "true" => ParsedAnnotationValueRootKind::Bool(true),
            "false" => ParsedAnnotationValueRootKind::Bool(false),
            "decimal_integer_literal" | "hex_integer_literal" => {
                ParsedAnnotationValueRootKind::Number(ParsedAnnotationNumberKind::Int)
            }
            "decimal_floating_point_literal" => {
                ParsedAnnotationValueRootKind::Number(ParsedAnnotationNumberKind::Double)
            }
            "unary_expression" => signed_number_kind(node)
                .map(ParsedAnnotationValueRootKind::Number)
                .unwrap_or(ParsedAnnotationValueRootKind::Expression),
            "string_literal" => parse_static_dart_string_literal(&value_source)
                .map(ParsedAnnotationValueRootKind::String)
                .unwrap_or(ParsedAnnotationValueRootKind::Expression),
            "list_literal" => ParsedAnnotationValueRootKind::List(collection_values(node, source)),
            "set_or_map_literal" => set_or_map_kind(node, source),
            "record_literal" => ParsedAnnotationValueRootKind::Record(record_values(node, source)),
            "const_object_expression" | "constructor_invocation" => {
                constructor_kind([node], source)
                    .unwrap_or(ParsedAnnotationValueRootKind::Expression)
            }
            "identifier" | "qualified" | "selector" => {
                ParsedAnnotationValueRootKind::Member(value_source.clone())
            }
            _ => ParsedAnnotationValueRootKind::Expression,
        }
    };

    ParsedAnnotationValue {
        source: value_source,
        span: value_span,
        kind,
    }
}

/// Returns the numeric kind for a directly signed numeric literal.
fn signed_number_kind(node: Node<'_>) -> Option<ParsedAnnotationNumberKind> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.is_named())
        .find_map(|child| match child.kind() {
            "decimal_integer_literal" | "hex_integer_literal" => {
                Some(ParsedAnnotationNumberKind::Int)
            }
            "decimal_floating_point_literal" => Some(ParsedAnnotationNumberKind::Double),
            _ => None,
        })
}

/// Parses direct collection elements without reparsing their source text.
fn collection_values(node: Node<'_>, source: &SourceText) -> Vec<ParsedAnnotationValue> {
    let mut cursor = node.walk();
    let children = node
        .children(&mut cursor)
        .filter(|child| child.is_named() && !child.is_extra() && child.kind() != "type_arguments")
        .collect::<Vec<_>>();
    let mut values = Vec::new();
    let mut index = 0;

    while let Some(child) = children.get(index).copied() {
        if matches!(child.kind(), "identifier" | "qualified") {
            let mut end = index + 1;
            while children
                .get(end)
                .is_some_and(|selector| selector.kind() == "selector")
            {
                end += 1;
            }
            if end > index + 1 {
                let last = children[end - 1];
                let span = TextRange::new(text_range(child).start(), text_range(last).end());
                let value_source = source.slice(span).unwrap_or_default().to_owned();
                let kind = constructor_kind(children[index..end].iter().copied(), source)
                    .unwrap_or_else(|| ParsedAnnotationValueRootKind::Member(value_source.clone()));
                values.push(ParsedAnnotationValue {
                    source: value_source,
                    span,
                    kind,
                });
                index = end;
                continue;
            }
        }

        let span = text_range(child);
        values.push(annotation_value(
            child,
            source,
            node_text(child, source),
            span,
            false,
        ));
        index += 1;
    }

    values
}

/// Parses direct values from a set literal.
fn set_values(node: Node<'_>, source: &SourceText) -> Vec<ParsedAnnotationValue> {
    collection_values(node, source)
}
