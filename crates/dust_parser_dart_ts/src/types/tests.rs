use dust_parser_dart::{DartLanguageVersion, ParseBackend, ParseOptions};
use dust_text::FileId;

use super::*;

fn field_type(text: &str) -> ParsedTypeSurface {
    let source = SourceText::new(
        FileId::new(51),
        format!("class C {{ final {text} value; }}"),
    );
    let parsed = crate::TreeSitterDartBackend::new().parse_file(
        &source,
        ParseOptions {
            language_version: DartLanguageVersion::DART_3_13,
            ..Default::default()
        },
    );
    assert_eq!(parsed.diagnostics, [], "{text}");
    let ty = parsed.library.classes[0].fields[0]
        .parsed_type
        .clone()
        .unwrap();
    assert_eq!(ty.source, text);
    assert_eq!(ty.span, TextRange::at("class C { final ".len(), text.len()));
    ty
}

#[test]
fn qualified_and_commented_types_keep_exact_names_and_nested_arguments() {
    for text in ["p.Item", "p /* prefix */ . Item /* no arguments */"] {
        assert_eq!(
            field_type(text).kind,
            ParsedTypeKind::Named {
                name: "p.Item".into(),
                args: Vec::new(),
            }
        );
    }
    let text = "p.Box /* arguments */ <List<p.Item?>>?";
    let ty = field_type(text);
    assert!(ty.nullable);
    let ParsedTypeKind::Named { name, args } = ty.kind else {
        panic!("named outer type")
    };
    assert_eq!(name, "p.Box");
    assert_eq!(args.len(), 1);
    assert_eq!(args[0].source, "List<p.Item?>");
    let ParsedTypeKind::Named { name, args } = &args[0].kind else {
        panic!("named list type")
    };
    assert_eq!(name, "List");
    assert_eq!(args.len(), 1);
    assert_eq!(args[0].source, "p.Item?");
    assert!(args[0].nullable);
    assert_eq!(
        args[0].kind,
        ParsedTypeKind::Named {
            name: "p.Item".into(),
            args: Vec::new()
        }
    );
    let ty = field_type("p.Box<String>");
    let ParsedTypeKind::Named { name, args } = ty.kind else {
        panic!("named box type")
    };
    assert_eq!(name, "p.Box");
    assert_eq!(args[0].kind, ParsedTypeKind::Builtin("String".into()));
}

#[test]
fn broad_classes_keep_every_qualified_field_without_neighbor_arguments() {
    let fields = (0..256)
        .map(|index| format!("final p.Item value{index};"))
        .collect::<String>();
    let text = format!("class C {{ {fields} final p.Box<int> last; }}");
    let source = SourceText::new(FileId::new(52), text);
    let parsed = crate::TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    let fields = &parsed.library.classes[0].fields;
    assert_eq!(fields.len(), 257);
    for (index, field) in fields[..256].iter().enumerate() {
        assert_eq!(field.name, format!("value{index}"));
        let ty = field.parsed_type.as_ref().unwrap();
        assert_eq!(ty.source, "p.Item");
        assert_eq!(source.slice(ty.span), Some("p.Item"));
        assert_eq!(
            ty.kind,
            ParsedTypeKind::Named {
                name: "p.Item".into(),
                args: Vec::new()
            }
        );
    }
    assert_eq!(
        fields[256].parsed_type.as_ref().unwrap().source,
        "p.Box<int>"
    );
}

/// The previous wider lookup is an oracle for recovery nodes and exact candidate order.
fn previous_lookup<'tree>(type_node: Node<'tree>, end: usize) -> Option<Node<'tree>> {
    let mut container = type_node.parent();
    while let Some(node) = container {
        if node.start_byte() <= type_node.start_byte() && node.end_byte() >= end {
            let mut cursor = node.walk();
            if let Some(args) = node.children(&mut cursor).find(|child| {
                child.kind() == "type_arguments"
                    && child.start_byte() >= type_node.end_byte()
                    && child.end_byte() <= end
            }) {
                return Some(args);
            }
        }
        container = node.parent();
    }
    None
}

#[test]
fn containment_matches_wider_lookup_for_valid_and_recovery_boundaries() {
    for text in [
        "class C { p.Item a; p.Box<List<p.Item?>> b; } p.Box<int> other;",
        "class C { p /* prefix */ . Box /* arguments */ <p.Item> value; }",
        "class C { p.Box<> value; p.Item other; }",
        "class C { p.Box<List<>> value; }",
        "class C { p.Box<p.Item value; }",
    ] {
        let tree = crate::PARSER
            .with(|parser| parser.borrow_mut().parse(text, None))
            .unwrap();
        let mut cursor = tree.walk();
        'walk: loop {
            let node = cursor.node();
            if node.kind() == "type_identifier" {
                for ancestor in std::iter::successors(node.parent(), |node| node.parent()) {
                    assert_eq!(
                        find_type_arguments_after(node, ancestor.end_byte()),
                        previous_lookup(node, ancestor.end_byte()),
                        "{text}: {node:?}"
                    );
                }
            }
            if cursor.goto_first_child() {
                continue;
            }
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    break 'walk;
                }
            }
        }
    }
}
