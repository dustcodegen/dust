use dust_parser_dart::{
    ParsedCombinatorKind, ParsedConditionalUri, ParsedNamespaceDirective, ParsedUriLiteral,
    ParsedVisibilityCombinator,
};
use dust_text::{SourceText, TextRange};
use tree_sitter::Node;

use crate::syntax::{
    direct_named_child, find_first_descendant, has_direct_child_kind, node_text, text_range,
};

/// Extracts the actual URI node, never a metadata argument or condition value.
pub(super) fn extract_uri_literal(node: Node<'_>, source: &SourceText) -> Option<ParsedUriLiteral> {
    find_first_descendant(node, "uri").map(|uri| literal(uri, source))
}

/// Preserves literals/conditions without decoding or selecting a host branch.
pub(super) fn extract_namespace(
    node: Node<'_>,
    source: &SourceText,
    combinators: Vec<ParsedVisibilityCombinator>,
) -> Option<ParsedNamespaceDirective> {
    let configurable = find_first_descendant(node, "configurable_uri");
    let default_uri = match configurable {
        Some(configurable) => direct_named_child(configurable, "uri")?,
        None => find_first_descendant(node, "uri")?,
    };
    let conditional_uris = configurable
        .map(|node| {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .filter(|child| child.kind() == "configuration_uri")
                .filter_map(|node| conditional_uri(node, source))
                .collect()
        })
        .unwrap_or_default();
    Some(ParsedNamespaceDirective {
        default_uri: literal(default_uri, source),
        conditional_uris,
        combinators,
    })
}

/// Retains one alternative's original parenthesized condition and URI source.
fn conditional_uri(node: Node<'_>, source: &SourceText) -> Option<ParsedConditionalUri> {
    let open = find_first_descendant(node, "(")?;
    let close = find_first_descendant(node, ")")?;
    let condition_span = TextRange::new(text_range(open).start(), text_range(close).end());
    let uri = direct_named_child(node, "uri")?;
    Some(ParsedConditionalUri {
        condition_source: source.slice(condition_span)?.to_owned(),
        condition_span,
        uri: literal(uri, source),
        span: text_range(node),
    })
}

/// Captures one literal's exact source spelling and byte range.
fn literal(node: Node<'_>, source: &SourceText) -> ParsedUriLiteral {
    ParsedUriLiteral {
        source: node_text(node, source),
        span: text_range(node),
    }
}

/// Collects visibility clauses once in written order, including repeated shows.
pub(super) fn extract_combinators(
    node: Node<'_>,
    source: &SourceText,
) -> Vec<ParsedVisibilityCombinator> {
    let mut result = Vec::new();
    collect_combinators(node, source, &mut result);
    result
}

/// Walks only directive syntax; comments cannot become combinator names.
fn collect_combinators(
    node: Node<'_>,
    source: &SourceText,
    result: &mut Vec<ParsedVisibilityCombinator>,
) {
    if node.kind() == "combinator" {
        let kind = if has_direct_child_kind(node, "show") {
            ParsedCombinatorKind::Show
        } else {
            ParsedCombinatorKind::Hide
        };
        let mut cursor = node.walk();
        let names = node
            .children(&mut cursor)
            .filter(|child| child.kind() == "identifier")
            .map(|child| node_text(child, source))
            .collect();
        result.push(ParsedVisibilityCombinator {
            kind,
            names,
            span: text_range(node),
        });
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor).filter(|child| child.is_named()) {
        collect_combinators(child, source, result);
    }
}
