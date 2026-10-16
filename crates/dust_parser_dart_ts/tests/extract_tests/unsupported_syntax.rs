//! AST acceptance must not silently enable unsupported generation.

use crate::support::parse;

#[test]
fn reports_exact_features_and_spans_for_new_constructor_shapes() {
    for (source, feature, range) in [
        (
            "class const User(final String name);",
            "primary constructors",
            "(final String name)",
        ),
        (
            "class Box<T>.named(final T value) { this; }",
            "primary constructors",
            ".named(final T value)",
        ),
        (
            "enum Color(final String hex) { red('#f00'); }",
            "primary constructors",
            "(final String hex)",
        ),
        ("class User { new(); }", "concise constructors", "new()"),
        (
            "class User { const new create(); }",
            "concise constructors",
            "const new create()",
        ),
        (
            "class User { factory() => User._(); User._(); }",
            "concise constructors",
            "factory()",
        ),
        (
            "class User { factory create() => User._(); User._(); }",
            "concise constructors",
            "factory create()",
        ),
        (
            "class User { factory create() = Other; }",
            "concise constructors",
            "factory create() = Other",
        ),
        (
            "enum Color { red; factory create() => red; }",
            "concise constructors",
            "factory create()",
        ),
    ] {
        let result = parse(313, source);
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message == format!("Dust does not support {feature} yet"))
            .unwrap_or_else(|| panic!("{source}: {:?}", result.diagnostics));
        assert_eq!(diagnostic.labels.len(), 1);
        let label = &diagnostic.labels[0];
        assert_eq!(label.file_id, dust_text::FileId::new(313));
        assert_eq!(
            &source[label.range.start().to_usize()..label.range.end().to_usize()],
            range
        );
        assert_eq!(
            diagnostic.notes,
            [
                "Check this file with `dart analyze`. Dust's bundled parser accepts this syntax, but code generation is not implemented yet.",
                "Use traditional in-body constructors with the class name and explicit fields.",
            ]
        );
    }
}

#[test]
fn dot_shorthands_have_specific_actionable_guidance() {
    for source in [
        "enum Status { ready } final status = .ready;",
        "enum Status { ready } Status current() => .ready;",
    ] {
        let result = parse(310, source);
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message == "Dust does not support dot shorthands yet")
            .unwrap();
        let label = &diagnostic.labels[0];
        assert_eq!(
            &source[label.range.start().to_usize()..label.range.end().to_usize()],
            ".ready"
        );
        assert_eq!(
            diagnostic.notes[1],
            "Use an explicit type name instead of a dot shorthand."
        );
    }
}

#[test]
fn traditional_factories_new_expressions_comments_and_strings_stay_supported() {
    let source = r#"
class User {
  // class Fake(final String id); const new create();
  const User();
  factory User.named() => new User();
  factory User.redirect() = User;
  static const text = 'factory create(); .ready';
}
"#;
    assert_eq!(parse(313, source).diagnostics, []);
}

#[test]
fn generic_constructor_receivers_are_not_dot_shorthands() {
    for source in [
        "void run() { List<int>.from(values); Future<void>.value(); }",
        "void run() { final values = List<int>.from(items); return Future<void>.value(); }",
        "void run() { final values = Map<String, List<int>>.from(items); }",
        include_str!(
            "../../../../examples/shopping_app/lib/features/cart/view_models/cart_view_model.dart"
        ),
        include_str!(
            "../../../../examples/shopping_app/lib/features/checkout/view_models/checkout_view_model.dart"
        ),
        include_str!(
            "../../../../examples/shopping_app/lib/features/integrations/view_models/external_integration_view_model.dart"
        ),
    ] {
        let result = parse(313, source);
        assert_eq!(result.diagnostics, [], "{source}");
    }
}

#[test]
fn comparison_and_arithmetic_operands_are_still_guarded_as_dot_shorthands() {
    let source = "class Value { const Value(); static const zero = Value(); bool operator >(Value other) => false; Value operator +(Value other) => this; } bool greater(Value value) => value > .zero; Value sum(Value value) => value + .zero;";
    let result = parse(313, source);
    assert_eq!(result.diagnostics.len(), 2, "{:?}", result.diagnostics);
    for diagnostic in result.diagnostics {
        assert_eq!(
            diagnostic.message,
            "Dust does not support dot shorthands yet"
        );
        let range = diagnostic.labels[0].range;
        assert_eq!(
            &source[range.start().to_usize()..range.end().to_usize()],
            ".zero"
        );
    }
}
