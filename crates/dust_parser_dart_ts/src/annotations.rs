use dust_parser_dart::{
    ParsedAnnotation, ParsedAnnotationArgument, ParsedAnnotationArguments,
    ParsedAnnotationNamedArgument,
};
use dust_text::{SourceText, TextRange, TextSize};
use tree_sitter::Node;

use self::values::annotation_value_from_container;
use crate::syntax::{direct_named_child, find_first_descendant_text, node_text, text_range};

/// Parser-owned annotation value extraction helpers.
mod values;

#[cfg(test)]
mod tests;

/// Extracts a structured annotation from a tree-sitter annotation node.
pub(crate) fn extract_annotation(node: Node<'_>, source: &SourceText) -> ParsedAnnotation {
    let (name, prefix, qualified_name) = node
        .child_by_field_name("name")
        .map(|name| annotation_name(name, source))
        .unwrap_or_default();
    let mut arguments_source = None;
    let mut parsed_arguments = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor).filter(|child| child.is_named()) {
        if matches!(child.kind(), "annotation_arguments" | "arguments") {
            arguments_source = Some(node_text(child, source));
            parsed_arguments = Some(extract_annotation_arguments(child, source));
        }
    }

    ParsedAnnotation {
        name,
        prefix,
        qualified_name,
        arguments_source,
        parsed_arguments,
        span: text_range(node),
    }
}

/// Splits an annotation name into short, prefix, and qualified forms.
fn annotation_name(node: Node<'_>, source: &SourceText) -> (String, Option<String>, String) {
    let qualified_name = node_text(node, source);
    if !matches!(node.kind(), "qualified" | "scoped_identifier") {
        return (qualified_name.clone(), None, qualified_name);
    }

    let mut identifiers = Vec::new();
    annotation_identifiers(node, source, &mut identifiers);
    let name = identifiers.pop().unwrap_or_default();
    let prefix = (!identifiers.is_empty()).then(|| identifiers.join("."));
    (name, prefix, qualified_name)
}

/// Reads only name identifiers; named comment extras never become a qualifier.
fn annotation_identifiers(node: Node<'_>, source: &SourceText, identifiers: &mut Vec<String>) {
    if matches!(node.kind(), "identifier" | "type_identifier") {
        identifiers.push(node_text(node, source));
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor).filter(|child| child.is_named()) {
        annotation_identifiers(child, source, identifiers);
    }
}

/// Extracts positional and named arguments from an annotation argument list.
pub(crate) fn extract_annotation_arguments(
    node: Node<'_>,
    source: &SourceText,
) -> ParsedAnnotationArguments {
    let mut arguments = ParsedAnnotationArguments::default();
    let mut cursor = node.walk();
    for child in node
        .children(&mut cursor)
        .filter(|child| child.is_named() && matches!(child.kind(), "argument" | "named_argument"))
    {
        let named = if child.kind() == "named_argument" {
            Some(child)
        } else {
            direct_named_child(child, "named_argument")
        };
        if let Some(named) = named {
            if let Some(argument) = extract_named_argument(child, named, source) {
                arguments.named.push(argument);
            }
        } else {
            let argument_source = node_text(child, source);
            arguments.positional.push(ParsedAnnotationArgument {
                value: annotation_value_from_container(
                    child,
                    argument_source.clone(),
                    text_range(child),
                    source,
                ),
                source: argument_source,
                span: text_range(child),
            });
        }
    }
    arguments
}

/// Extracts one named annotation argument and its value span.
fn extract_named_argument(
    argument_node: Node<'_>,
    named_node: Node<'_>,
    source: &SourceText,
) -> Option<ParsedAnnotationNamedArgument> {
    let label = direct_named_child(named_node, "label")?;
    let name = find_first_descendant_text(label, source, &["identifier"])?;
    let value_span = named_value_span(named_node, label, source);
    let value_source = source.slice(value_span).unwrap_or_default().to_owned();

    Some(ParsedAnnotationNamedArgument {
        name,
        source: node_text(argument_node, source),
        value: annotation_value_from_container(
            named_node,
            value_source.clone(),
            value_span,
            source,
        ),
        value_source,
        span: text_range(argument_node),
        value_span,
    })
}

/// Returns the source span containing a named argument value without the label.
fn named_value_span(named_node: Node<'_>, label_node: Node<'_>, source: &SourceText) -> TextRange {
    let mut start = label_node.end_byte();
    let mut end = named_node.end_byte();
    let bytes = source.as_str().as_bytes();

    while start < end && bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
        start += 1;
    }
    if bytes.get(start) == Some(&b':') {
        start += 1;
    }
    while start < end && bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
        start += 1;
    }
    while end > start
        && bytes
            .get(end.saturating_sub(1))
            .is_some_and(u8::is_ascii_whitespace)
    {
        end -= 1;
    }

    TextRange::new(TextSize::from(start), TextSize::from(end))
}

/// Extracts annotations attached directly to a class or mixin member node.
pub(crate) fn extract_member_annotations(
    node: Node<'_>,
    source: &SourceText,
) -> Vec<ParsedAnnotation> {
    extract_direct_annotations(node, source)
}

/// Extracts annotation children that are direct named children of a node.
pub(crate) fn extract_direct_annotations(
    node: Node<'_>,
    source: &SourceText,
) -> Vec<ParsedAnnotation> {
    let mut annotations = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor).filter(|child| child.is_named()) {
        if child.kind() == "annotation" {
            annotations.push(extract_annotation(child, source));
        }
    }
    annotations
}
