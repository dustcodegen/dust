use dust_parser_dart::{ParsedLexicalInitializer, ParsedLexicalInitializerKind, ParsedLexicalName};
use dust_text::{SourceText, TextRange};
use tree_sitter::Node;

use crate::{annotations::extract_annotation_arguments, syntax::text_range};

use super::identifier;

/// Keep only direct initializer nodes after `=`, excluding extra/comment nodes.
pub(super) fn extract(node: Node<'_>, source: &SourceText) -> Option<ParsedLexicalInitializer> {
    let mut cursor = node.walk();
    let mut after_equal = false;
    let values = node
        .children(&mut cursor)
        .filter(|child| {
            if child.kind() == "=" {
                after_equal = true;
                return false;
            }
            after_equal && child.is_named() && !child.is_extra()
        })
        .collect::<Vec<_>>();
    let span = TextRange::new(
        text_range(*values.first()?).start(),
        text_range(*values.last()?).end(),
    );
    let mut names = Vec::new();
    let mut arguments = None;
    let recognized = values
        .iter()
        .all(|node| reference(*node, source, &mut names, &mut arguments));
    let kind = if !recognized || names.is_empty() {
        ParsedLexicalInitializerKind::Opaque
    } else if let Some(arguments) = arguments {
        ParsedLexicalInitializerKind::Constructor {
            callee: names,
            arguments: extract_annotation_arguments(arguments, source),
        }
    } else {
        ParsedLexicalInitializerKind::Reference(names)
    };
    Some(ParsedLexicalInitializer {
        source: source.slice(span).unwrap_or_default().to_owned(),
        span,
        kind,
    })
}

/// Follow only direct callee/reference structure; nested expressions stay opaque.
fn reference<'tree>(
    node: Node<'tree>,
    source: &SourceText,
    names: &mut Vec<ParsedLexicalName>,
    arguments: &mut Option<Node<'tree>>,
) -> bool {
    if node.is_extra() {
        return true;
    }
    if arguments.is_some() {
        return false;
    }
    match node.kind() {
        "identifier" | "type_identifier" | "new" => {
            names.push(identifier(node, source));
            true
        }
        "arguments" => {
            *arguments = Some(node);
            true
        }
        "type_arguments" | "." | "const" | "const_builtin" => true,
        "selector"
        | "unconditional_assignable_selector"
        | "argument_part"
        | "qualified"
        | "scoped_identifier"
        | "constructor_invocation"
        | "const_object_expression" => {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .all(|child| reference(child, source, names, arguments))
        }
        _ => false,
    }
}
