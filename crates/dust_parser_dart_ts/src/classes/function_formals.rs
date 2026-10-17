use dust_parser_dart::{
    ParameterKind, ParsedFunctionFormalParameterSurface, ParsedFunctionFormalTypeSurface,
    ParsedLexicalName,
};
use dust_text::{SourceText, TextRange};
use tree_sitter::Node;

use crate::{
    annotations::extract_annotation,
    syntax::{direct_named_child, node_text, text_range},
    types::extract_type_before,
};

use super::{constructors, generics, parse_text};

/// Retains only an argument list owned by the declaring parameter itself.
pub(crate) fn extract(
    node: Node<'_>,
    source: &SourceText,
) -> Option<ParsedFunctionFormalTypeSurface> {
    let name = parse_text::parameter_name_node(node)?;
    let parameters = direct_named_child(node, "formal_parameter_list")
        .filter(|parameters| parameters.start_byte() >= name.end_byte())?;
    let mut cursor = node.walk();
    let nullable = node.children(&mut cursor).any(|child| {
        matches!(child.kind(), "nullable_type" | "?") && child.start_byte() >= parameters.end_byte()
    });
    let mut signature_parameters = Vec::new();
    collect(
        parameters,
        source,
        &mut signature_parameters,
        ParameterKind::Positional,
    );
    Some(ParsedFunctionFormalTypeSurface {
        name: ParsedLexicalName {
            name: node_text(name, source),
            span: text_range(name),
        },
        return_type: extract_type_before(node, name.start_byte(), source),
        type_parameters: generics::extract_class_type_parameters(node, source),
        parameters: signature_parameters,
        nullable,
        span: text_range(node),
    })
}

/// Stops at each direct formal; nested signatures are extracted independently.
fn collect(
    node: Node<'_>,
    source: &SourceText,
    parameters: &mut Vec<ParsedFunctionFormalParameterSurface>,
    kind: ParameterKind,
) {
    let mut annotations = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "annotation" => annotations.push(extract_annotation(child, source)),
            "optional_formal_parameters" => {
                collect(
                    child,
                    source,
                    parameters,
                    parse_text::optional_parameter_kind(child),
                );
            }
            "formal_parameter" | "default_formal_parameter" => {
                let name = parse_text::parameter_name_node(child).unwrap_or(child);
                let mut parameter = constructors::extract_formal_parameter(child, source, kind);
                let span = TextRange::new(
                    annotations
                        .first()
                        .map_or(parameter.span.start(), |annotation| annotation.span.start()),
                    parameter.span.end(),
                );
                annotations.append(&mut parameter.annotations);
                parameters.push(ParsedFunctionFormalParameterSurface {
                    name: ParsedLexicalName {
                        name: parameter.name,
                        span: text_range(name),
                    },
                    parsed_type: parameter.parsed_type,
                    function_formal: parameter.function_formal,
                    kind,
                    is_required: parameter.is_required,
                    annotations: std::mem::take(&mut annotations),
                    default_value_source: parameter.default_value_source,
                    span,
                });
            }
            _ => {}
        }
    }
}
