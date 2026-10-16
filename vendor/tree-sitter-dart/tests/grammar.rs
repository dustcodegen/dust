//! Grammar acceptance and AST contracts, not Dust generation support.

use tree_sitter::{Parser, Tree};

/// Parses with the same runtime used by Dust, without changing the input.
fn parse(source: &str) -> Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_dart::LANGUAGE.into())
        .expect("pinned grammar must load in Dust's runtime");
    let tree = parser
        .parse(source, None)
        .expect("parse must return a tree");
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    tree
}

#[test]
fn traditional_constructor_shape_is_explicit() {
    let tree = parse("class User { const User(this.name); final String name; }");
    assert_eq!(
        tree.root_node().to_sexp(),
        "(program (class_definition name: (identifier) body: (class_body (declaration (constant_constructor_signature (const_builtin) name: (identifier) (formal_parameter_list (formal_parameter (constructor_param (this) (identifier)))))) (declaration (final_builtin) (type_identifier) (initialized_identifier_list (initialized_identifier (identifier)))))))",
    );
}

#[test]
fn primary_header_shape_is_explicit() {
    let tree = parse("class const User(final String name);");
    assert_eq!(
        tree.root_node().to_sexp(),
        "(program (class_definition (const_builtin) name: (identifier) primary_constructor: (primary_constructor parameters: (formal_parameter_list (formal_parameter (final_builtin) (type_identifier) name: (identifier))))))",
    );
}

#[test]
fn named_concise_constructor_shape_is_explicit() {
    let tree = parse("class User { const new create(this.name); final String name; }");
    assert_eq!(
        tree.root_node().to_sexp(),
        "(program (class_definition name: (identifier) body: (class_body (declaration (constant_constructor_signature (const_builtin) name: (identifier) (formal_parameter_list (formal_parameter (constructor_param (this) (identifier)))))) (declaration (final_builtin) (type_identifier) (initialized_identifier_list (initialized_identifier (identifier)))))))",
    );
}

#[test]
fn malformed_source_is_not_accepted() {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_dart::LANGUAGE.into())
        .unwrap();
    assert!(parser
        .parse("class Broken {", None)
        .unwrap()
        .root_node()
        .has_error());
}

#[test]
fn generic_named_primary_body_and_enum_parse_without_errors() {
    for source in [
        "class Box<T>.named(final T value, {final int count = 0}) { this : assert(count >= 0); }",
        "enum Color(final String hex) { red('#f00'), blue('#00f'); }",
        "class A(final int a); class B(super.a, final int b) extends A;",
    ] {
        parse(source);
    }
}
