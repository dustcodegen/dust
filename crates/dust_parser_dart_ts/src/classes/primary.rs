//! Lower declaring parameters to ordinary field facts; plain parameters stay plain.

use dust_parser_dart::{ParsedConstructorParamSurface, ParsedFieldSurface};
use tree_sitter::Node;

use super::parse_text::parameter_name_node;
use crate::syntax::{has_direct_child_kind, text_range};

#[cfg(test)]
mod tests;

/// Extracts only primary parameters that induce instance fields.
pub(super) fn extract_declaring_fields(
    primary: Node<'_>,
    params: &[ParsedConstructorParamSurface],
) -> Vec<ParsedFieldSurface> {
    let mut fields = Vec::new();
    collect_declaring_fields(primary, params, &mut fields);
    fields
}

/// Traverses parameter containers, never nested callbacks or default expressions.
fn collect_declaring_fields(
    node: Node<'_>,
    params: &[ParsedConstructorParamSurface],
    out: &mut Vec<ParsedFieldSurface>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "formal_parameter" => {
                if !has_direct_child_kind(child, "final_builtin")
                    && !has_direct_child_kind(child, "inferred_type")
                {
                    continue;
                }
                let Some(name) = parameter_name_node(child).filter(|name| !name.is_missing())
                else {
                    continue;
                };
                let Some(param) = params.iter().find(|param| {
                    param.span.start().to_usize() <= child.start_byte()
                        && param.span.end().to_usize() >= child.end_byte()
                }) else {
                    continue;
                };
                out.push(ParsedFieldSurface {
                    name: param.name.clone(),
                    annotations: param.annotations.clone(),
                    type_source: param.type_source.clone(),
                    parsed_type: param.parsed_type.clone(),
                    // A parameter default does not become a field initializer.
                    has_default: false,
                    span: text_range(name),
                });
            }
            "formal_parameter_list" | "optional_formal_parameters" | "default_formal_parameter" => {
                collect_declaring_fields(child, params, out);
            }
            _ => {}
        }
    }
}
