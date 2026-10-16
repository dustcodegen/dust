//! Extracting extension and extension type declarations.

use super::*;

/// Extracts an extension surface from an extension declaration.
pub(super) fn extract_extension(node: Node<'_>, source: &SourceText) -> ParsedExtensionSurface {
    let name = node
        .child_by_field_name("name")
        .map(|name| node_text(name, source));
    let parsed_on_type = node
        .child_by_field_name("class")
        .and_then(|ty| extract_type_node(ty, ty.end_byte(), source));
    let on_type_source = parsed_on_type.as_ref().map(|ty| ty.source.clone());

    ParsedExtensionSurface {
        name,
        on_type_source,
        parsed_on_type,
        annotations: direct_annotations(node, source),
        span: text_range(node),
    }
}

/// Extracts an extension type and its representation metadata.
pub(super) fn extract_extension_type(
    node: Node<'_>,
    source: &SourceText,
) -> ParsedExtensionTypeSurface {
    let representation = node.child_by_field_name("representation");
    let name = extension_type_name(node, representation, source).unwrap_or_default();
    let representation_name = representation
        .and_then(|representation| representation.child_by_field_name("name"))
        .map(|name| node_text(name, source))
        .unwrap_or_default();
    let parsed_representation_type = representation
        .and_then(|representation| representation.child_by_field_name("type"))
        .and_then(|ty| extract_type_node(ty, ty.end_byte(), source));
    let representation_type_source = parsed_representation_type
        .as_ref()
        .map(|ty| ty.source.clone());

    ParsedExtensionTypeSurface {
        name,
        representation_name,
        representation_type_source,
        parsed_representation_type,
        annotations: direct_annotations(node, source),
        span: text_range(node),
    }
}

/// Preserves the existing name fact, including type parameters and a named representation.
fn extension_type_name(
    node: Node<'_>,
    representation: Option<Node<'_>>,
    source: &SourceText,
) -> Option<String> {
    let name = node.child_by_field_name("name")?;
    let Some(representation) = representation.filter(|_| name.kind() != "extension_type_name")
    else {
        return Some(node_text(name, source));
    };
    let mut end = node
        .child_by_field_name("type_parameters")
        .map_or(name.end_byte(), |params| params.end_byte());
    let boundary = direct_child(representation, "(")?.start_byte();
    let mut cursor = representation.walk();
    if let Some(constructor) = representation
        .named_children(&mut cursor)
        .find(|child| child.kind() == "identifier" && child.end_byte() <= boundary)
    {
        end = constructor.end_byte();
    }
    source
        .as_str()
        .get(name.start_byte()..end)
        .map(ToOwned::to_owned)
}

/// Missing representation metadata must not reconstruct a different declaration name.
#[cfg(test)]
mod parity_tests {
    use super::*;
    use dust_text::FileId;
    use tree_sitter::Parser;

    #[test]
    fn missing_representation_keeps_the_grammar_name() {
        let source = SourceText::new(FileId::new(0), "extension type Box<T>(T value) {}");
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_dart::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source.as_str(), None).unwrap();
        assert!(!tree.root_node().has_error());
        let declaration = tree.root_node().named_child(0).unwrap();
        let name = declaration.child_by_field_name("name").unwrap();
        assert_eq!(
            extension_type_name(declaration, None, &source),
            Some(node_text(name, &source))
        );
    }
}
