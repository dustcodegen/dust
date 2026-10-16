use dust_parser_dart::{ParsedCombinatorKind, ParsedDirective, ParsedVisibilityCombinator};
use dust_text::SourceText;
use tree_sitter::Node;

use crate::{
    annotations::extract_member_annotations,
    syntax::{
        direct_named_child, find_first_descendant_text, has_descendant_kind, node_text, text_range,
        unquote,
    },
};

/// Ordered, source-preserving directive inputs.
mod namespace;
use namespace::{extract_combinators, extract_namespace, extract_uri_literal};

/// Extracts Dart library directives from the root node.
pub(crate) fn extract_directives(root: Node<'_>, source: &SourceText) -> Vec<ParsedDirective> {
    let mut directives = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor).filter(|node| node.is_named()) {
        match child.kind() {
            "library_name" => directives.push(ParsedDirective::Library {
                name: direct_named_child(child, "dotted_identifier_list")
                    .map(|name| node_text(name, source)),
                annotations: extract_member_annotations(child, source),
                span: text_range(child),
            }),
            "import_or_export" => {
                if let Some(import) = direct_named_child(child, "library_import") {
                    let import_specification = direct_named_child(import, "import_specification");
                    let combinators = extract_combinators(import, source);
                    let (show, hide) = legacy_import_combinators(&combinators);
                    directives.push(ParsedDirective::Import {
                        namespace: extract_namespace(import, source, combinators),
                        uri: find_first_descendant_text(import, source, &["uri", "string_literal"])
                            .map(unquote)
                            .unwrap_or_default(),
                        prefix: import_specification
                            .and_then(|specification| {
                                specification
                                    .child_by_field_name("alias")
                                    .or_else(|| direct_named_child(specification, "identifier"))
                            })
                            .map(|alias| node_text(alias, source))
                            .filter(|prefix| !prefix.is_empty()),
                        show,
                        hide,
                        is_deferred: has_descendant_kind(import, "deferred"),
                        span: text_range(child),
                    });
                } else if let Some(export) = direct_named_child(child, "library_export") {
                    directives.push(ParsedDirective::Export {
                        namespace: extract_namespace(
                            export,
                            source,
                            extract_combinators(export, source),
                        ),
                        uri: find_first_descendant_text(export, source, &["uri", "string_literal"])
                            .map(unquote)
                            .unwrap_or_default(),
                        span: text_range(child),
                    });
                }
            }
            "part_directive" => directives.push(ParsedDirective::Part {
                uri_literal: extract_uri_literal(child, source),
                uri: find_first_descendant_text(child, source, &["uri", "string_literal"])
                    .map(unquote)
                    .unwrap_or_default(),
                span: text_range(child),
            }),
            "part_of_directive" => directives.push(ParsedDirective::PartOf {
                uri_literal: extract_uri_literal(child, source),
                library_name: direct_named_child(child, "dotted_identifier_list")
                    .map(|name| node_text(name, source))
                    .filter(|name| !name.is_empty()),
                uri: find_first_descendant_text(child, source, &["uri", "string_literal"])
                    .map(unquote),
                span: text_range(child),
            }),
            _ => {}
        }
    }

    directives
}

/// Keeps the legacy flattened lists without redefining their compatibility contract.
fn legacy_import_combinators(
    combinators: &[ParsedVisibilityCombinator],
) -> (Vec<String>, Vec<String>) {
    let mut show = Vec::new();
    let mut hide = Vec::new();
    for combinator in combinators {
        let target = match combinator.kind {
            ParsedCombinatorKind::Show => &mut show,
            ParsedCombinatorKind::Hide => &mut hide,
        };
        target.extend(combinator.names.iter().cloned());
    }
    (show, hide)
}
