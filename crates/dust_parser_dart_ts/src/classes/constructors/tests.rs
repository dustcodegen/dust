use super::*;
use dust_text::FileId;

#[test]
fn missing_constructor_signature_preserves_a_complete_absent_fact_surface() {
    let source = SourceText::new(FileId::new(134), "class Plain {}");
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_dart::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source.as_str(), None).unwrap();
    assert_eq!(
        extract_constructor(tree.root_node(), &[], &source),
        ParsedConstructorSurface {
            name: None,
            is_factory: false,
            annotations: vec![],
            redirected_target_source: None,
            redirected_target_name: None,
            redirected_target: None,
            params: vec![],
            span: text_range(tree.root_node()),
        }
    );
}
