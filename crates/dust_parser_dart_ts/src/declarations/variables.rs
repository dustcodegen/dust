//! Extracting top-level variable declarations in their three Dart spellings.

use super::*;

/// Extracts initialized top-level variable declarations.
pub(super) fn extract_initialized_variables(
    list: Node<'_>,
    type_node: Option<Node<'_>>,
    annotations: &[ParsedAnnotation],
    source: &SourceText,
) -> Vec<ParsedTopLevelVariableSurface> {
    let parsed_type = type_node.and_then(|ty| extract_type_node(ty, list.start_byte(), source));
    let type_source = parsed_type.as_ref().map(|ty| ty.source.clone());
    let mut variables = Vec::new();
    let mut cursor = list.walk();
    for initialized in list
        .children(&mut cursor)
        .filter(|child| child.is_named() && child.kind() == "initialized_identifier")
    {
        let name = initialized
            .child_by_field_name("name")
            .or_else(|| direct_named_child(initialized, "identifier"))
            .map(|name| node_text(name, source))
            .unwrap_or_default();
        let value = initializer_node(initialized);
        let initializer_source = value.map(|value| node_text(value, source));
        let initializer_span = value.map(text_range);
        variables.push(ParsedTopLevelVariableSurface {
            name,
            type_source: type_source.clone(),
            parsed_type: parsed_type.clone(),
            initializer_source,
            initializer_span,
            annotations: annotations.to_vec(),
            span: text_range(initialized),
        });
    }
    variables
}

/// Extracts static final top-level variable declarations.
pub(super) fn extract_static_final_variables(
    list: Node<'_>,
    type_node: Option<Node<'_>>,
    annotations: &[ParsedAnnotation],
    source: &SourceText,
) -> Vec<ParsedTopLevelVariableSurface> {
    let parsed_type = type_node.and_then(|ty| extract_type_node(ty, list.start_byte(), source));
    let type_source = parsed_type.as_ref().map(|ty| ty.source.clone());
    let mut variables = Vec::new();
    let mut cursor = list.walk();
    for declaration in list
        .children(&mut cursor)
        .filter(|child| child.is_named() && child.kind() == "static_final_declaration")
    {
        let name = declaration
            .child_by_field_name("name")
            .or_else(|| direct_named_child(declaration, "identifier"))
            .map(|name| node_text(name, source))
            .unwrap_or_default();
        let value = initializer_node(declaration);
        let initializer_source = value.map(|value| node_text(value, source));
        let initializer_span = value.map(text_range);
        variables.push(ParsedTopLevelVariableSurface {
            name,
            type_source: type_source.clone(),
            parsed_type: parsed_type.clone(),
            initializer_source,
            initializer_span,
            annotations: annotations.to_vec(),
            span: text_range(declaration),
        });
    }
    variables
}

/// Preserves the existing first-expression-node fact when the grammar omits fields.
fn initializer_node(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("value").or_else(|| {
        let equals = direct_child(node, "=")?;
        let mut cursor = node.walk();
        node.named_children(&mut cursor).find(|child| {
            child.start_byte() >= equals.end_byte()
                && !matches!(child.kind(), "comment" | "documentation_comment")
        })
    })
}

/// Extracts external top-level variables that have no initializer node.
pub(super) fn extract_external_variables(
    list: Node<'_>,
    type_node: Option<Node<'_>>,
    annotations: &[ParsedAnnotation],
    source: &SourceText,
) -> Vec<ParsedTopLevelVariableSurface> {
    let parsed_type = type_node.and_then(|ty| extract_type_node(ty, list.start_byte(), source));
    let type_source = parsed_type.as_ref().map(|ty| ty.source.clone());
    let mut variables = Vec::new();
    let mut cursor = list.walk();
    for identifier in list
        .children(&mut cursor)
        .filter(|child| child.is_named() && child.kind() == "identifier")
    {
        variables.push(ParsedTopLevelVariableSurface {
            name: node_text(identifier, source),
            type_source: type_source.clone(),
            parsed_type: parsed_type.clone(),
            initializer_source: None,
            initializer_span: None,
            annotations: annotations.to_vec(),
            span: text_range(identifier),
        });
    }
    variables
}
