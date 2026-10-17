use dust_parser_dart::{ParameterKind, ParsedConstructorParamSurface};
use tree_sitter::Parser;

use super::extract_declaring_fields;
use crate::syntax::{find_first_descendant, text_range};

#[test]
fn declaring_fields_need_matching_extracted_parameter_facts() {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_dart::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse("void f(final int value) {}", None).unwrap();
    let list = find_first_descendant(tree.root_node(), "formal_parameter_list").unwrap();
    assert_eq!(extract_declaring_fields(list, &[]), []);
}

#[test]
fn malformed_declaring_parameters_do_not_invent_field_names() {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_dart::LANGUAGE.into())
        .unwrap();
    let tree = parser
        .parse("void f(final void Function()) {}", None)
        .unwrap();
    assert!(tree.root_node().has_error());
    let list = find_first_descendant(tree.root_node(), "formal_parameter_list").unwrap();
    let recovered = ParsedConstructorParamSurface {
        name: String::new(),
        annotations: Vec::new(),
        type_source: None,
        parsed_type: None,
        kind: ParameterKind::Positional,
        is_required: true,
        has_default: false,
        default_value_source: None,
        span: text_range(list),
    };
    assert_eq!(extract_declaring_fields(list, &[recovered]), []);
}
