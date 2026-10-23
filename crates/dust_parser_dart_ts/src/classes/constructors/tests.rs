use super::*;
use dust_text::FileId;

#[test]
fn required_constructor_presence_survives_the_linked_grammar_layout() {
    let source = SourceText::new(
        FileId::new(138),
        "class Shape { const Shape({@Tag('page') required this.page}); }",
    );
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_dart::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source.as_str(), None).unwrap();
    assert!(!tree.root_node().has_error());
    let constructor = find_first_descendant(tree.root_node(), "constant_constructor_signature")
        .or_else(|| find_first_descendant(tree.root_node(), "constructor_signature"))
        .unwrap();
    let constructor = extract_constructor(constructor, &[], &source);
    assert!(constructor.is_const);
    assert!(constructor.params[0].is_required);
}

#[test]
fn const_redirecting_factory_retains_both_independent_modifiers() {
    let source = SourceText::new(
        FileId::new(139),
        "abstract class Shape { const factory Shape() = Circle; } class Circle implements Shape { const Circle(); }",
    );
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_dart::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source.as_str(), None).unwrap();
    assert!(!tree.root_node().has_error());
    let constructor = find_first_descendant(
        tree.root_node(),
        "redirecting_factory_constructor_signature",
    )
    .unwrap();
    let constructor = extract_constructor(constructor, &[], &source);
    assert!(constructor.is_factory);
    assert!(constructor.is_const);
}

#[test]
fn const_default_expression_does_not_mark_the_constructor_const() {
    let source = SourceText::new(
        FileId::new(140),
        "class Shape { Shape([Object value = const Object()]); }",
    );
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_dart::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source.as_str(), None).unwrap();
    assert!(!tree.root_node().has_error());
    let constructor = find_first_descendant(tree.root_node(), "constructor_signature").unwrap();
    assert!(!extract_constructor(constructor, &[], &source).is_const);
}

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
            is_const: false,
            annotations: vec![],
            redirected_target_source: None,
            redirected_target_name: None,
            redirected_target: None,
            params: vec![],
            span: text_range(tree.root_node()),
        }
    );
}
