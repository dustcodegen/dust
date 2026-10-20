use dust_parser_dart::{ParsedRedirectIdentifierSurface, ParsedRedirectTargetSurface};
use dust_text::{SourceText, TextRange};
use tree_sitter::Node;

use crate::{
    syntax::{direct_named_child, node_text, text_range},
    types::extract_type_node,
};

/// Separates the target type application from a trailing constructor identifier.
pub(super) fn extract_redirect_application(
    signature: Node<'_>,
    source: &SourceText,
) -> Option<ParsedRedirectTargetSurface> {
    if signature.kind() != "redirecting_factory_constructor_signature" {
        return None;
    }
    let boundary = direct_named_child(signature, "formal_parameter_list")?.end_byte();
    let mut cursor = signature.walk();
    let children = signature
        .children(&mut cursor)
        .filter(|child| child.start_byte() >= boundary && !child.is_extra())
        .collect::<Vec<_>>();
    let first = *children.iter().find(|child| {
        matches!(
            child.kind(),
            "type_identifier" | "function_type" | "record_type"
        )
    })?;
    let constructor = children
        .iter()
        .find(|child| {
            child.start_byte() >= first.end_byte() && matches!(child.kind(), "identifier" | "new")
        })
        .map(|child| ParsedRedirectIdentifierSurface {
            name: node_text(*child, source),
            span: text_range(*child),
        });
    let end = children
        .iter()
        .filter(|child| child.is_named() && child.start_byte() >= first.start_byte())
        .take_while(|child| {
            constructor
                .as_ref()
                .is_none_or(|suffix| text_range(**child).start() < suffix.span.start())
        })
        .last()?
        .end_byte();
    let ty = extract_type_node(first, end, source)?;
    let type_name_parts = children
        .iter()
        .filter(|child| child.kind() == "type_identifier" && child.end_byte() <= end)
        .map(|child| ParsedRedirectIdentifierSurface {
            name: node_text(*child, source),
            span: text_range(*child),
        })
        .collect();
    let span = TextRange::new(
        ty.span.start(),
        constructor
            .as_ref()
            .map_or(ty.span.end(), |suffix| suffix.span.end()),
    );
    Some(ParsedRedirectTargetSurface {
        ty,
        type_name_parts,
        constructor,
        span,
    })
}
