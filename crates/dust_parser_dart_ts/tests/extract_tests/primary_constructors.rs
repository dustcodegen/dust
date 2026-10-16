//! Complete field/constructor facts are required before enabling generation.

use crate::support::parse;
use dust_parser_dart::ParameterKind;

#[test]
fn declaring_parameters_induce_fields_but_plain_parameters_do_not() {
    let source = "class const User(final String id, String input, {final int page = 1, final String? note});";
    let result = parse(313, source);
    assert_eq!(result.diagnostics, []);
    let class = &result.library.classes[0];
    assert_eq!(class.name, "User");
    assert_eq!(
        class
            .fields
            .iter()
            .map(|field| (
                field.name.as_str(),
                field.type_source.as_deref(),
                field.has_default
            ))
            .collect::<Vec<_>>(),
        [
            ("id", Some("String"), false),
            ("page", Some("int"), false),
            ("note", Some("String?"), false),
        ]
    );
    assert_eq!(class.constructors.len(), 1);
    let constructor = &class.constructors[0];
    assert_eq!(constructor.name, None);
    assert!(!constructor.is_factory);
    assert_eq!(
        constructor
            .params
            .iter()
            .map(|param| (
                param.name.as_str(),
                param.type_source.as_deref(),
                param.kind,
                param.default_value_source.as_deref()
            ))
            .collect::<Vec<_>>(),
        [
            ("id", Some("String"), ParameterKind::Positional, None),
            ("input", Some("String"), ParameterKind::Positional, None),
            ("page", Some("int"), ParameterKind::Named, Some("1")),
            ("note", Some("String?"), ParameterKind::Named, None),
        ]
    );
}

#[test]
fn named_generic_headers_and_body_metadata_keep_distinct_owners() {
    let source = r#"
@ClassTag()
class Bundle<T>.named(@FieldTag('items') final List<T?> items, {required var int count}) {
  @ConstructorTag()
  this : assert(count >= 0);
  final String extra = 'saved';
  int size() => count;
}
"#;
    let result = parse(313, source);
    assert_eq!(result.diagnostics, []);
    let class = &result.library.classes[0];
    assert_eq!(
        class
            .annotations
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["ClassTag"]
    );
    assert_eq!(
        class
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["items", "count", "extra"]
    );
    assert_eq!(class.fields[0].type_source.as_deref(), Some("List<T?>"));
    assert_eq!(class.fields[0].annotations[0].name, "FieldTag");
    assert_eq!(
        class.fields[0].annotations[0].arguments_source.as_deref(),
        Some("('items')")
    );
    assert_eq!(class.fields[1].type_source.as_deref(), Some("int"));
    assert_eq!(class.constructors.len(), 1);
    let constructor = &class.constructors[0];
    assert_eq!(constructor.name.as_deref(), Some("named"));
    assert_eq!(
        constructor
            .annotations
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["ConstructorTag"]
    );
    assert_eq!(
        constructor.params[0].annotations[0],
        class.fields[0].annotations[0]
    );
    assert_eq!(constructor.params[1].kind, ParameterKind::Named);
    assert_eq!(
        class
            .methods
            .iter()
            .map(|method| method.name.as_str())
            .collect::<Vec<_>>(),
        ["size"]
    );
}

#[test]
fn existing_field_and_super_formals_do_not_induce_duplicate_fields() {
    let source =
        "class Child(super.id, this.name, final int count) extends Parent { final String name; }";
    let result = parse(313, source);
    assert_eq!(result.diagnostics, []);
    let class = &result.library.classes[0];
    assert_eq!(class.superclass_name.as_deref(), Some("Parent"));
    assert_eq!(
        class
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["count", "name"]
    );
    assert_eq!(
        class.constructors[0]
            .params
            .iter()
            .map(|param| (param.name.as_str(), param.type_source.as_deref()))
            .collect::<Vec<_>>(),
        [("id", None), ("name", None), ("count", Some("int")),]
    );
}

#[test]
fn every_primary_header_shape_requires_the_right_language_version() {
    for declaration in [
        "class User(final String name);",
        "class const User(final String name);",
        "class User<T>.named(final T value);",
        "enum Color(final String hex) { red('#f00'); }",
        "mixin class Shared();",
    ] {
        let source = format!("// @dart=3.12\n{declaration}");
        let result = parse(312, &source);
        let diagnostic = result.diagnostics.iter().find(|diagnostic| {
            diagnostic.message == "primary constructors require Dart 3.13 or newer; current language version is 3.12"
        }).unwrap_or_else(|| panic!("{source}: {:?}", result.diagnostics));
        assert_eq!(diagnostic.labels.len(), 1);
        assert_eq!(diagnostic.labels[0].file_id, dust_text::FileId::new(312));
    }
}

#[test]
fn primary_text_in_comments_and_strings_is_not_a_language_feature() {
    let result = parse(
        312,
        "// @dart=3.12\n// class Example(final String id);\nclass User { const User(); static const text = 'class Fake(int value);'; }",
    );
    assert_eq!(result.diagnostics, []);
}
