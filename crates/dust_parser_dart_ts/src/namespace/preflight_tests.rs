use dust_text::FileId;

use super::*;

fn project(text: &str, version: DartLanguageVersion, annotations: &[&str]) -> bool {
    let source = SourceText::new(FileId::new(41), text);
    let tree = crate::PARSER
        .with(|parser| parser.borrow_mut().parse(text, None))
        .unwrap();
    assert!(!tree.root_node().has_error(), "{text}");
    can_project(tree.root_node(), &source, version, annotations)
}

#[test]
fn represented_metadata_requires_complete_facts_including_nested_callbacks() {
    for text in [
        "@p /* prefix */ . Marker() class C {}",
        "class C { @Marker() final int value = 1; }",
        "class C { void method(void callback(@Marker() int value)) {} }",
        "class C { C(void callback(@Marker() int value)); }",
        "void function(void callback(@Marker() int value)) {}",
        "typedef Callback = void Function(@Marker() int value);",
    ] {
        for version in [
            DartLanguageVersion::DART_3_0,
            DartLanguageVersion::DART_3_12,
        ] {
            assert!(!project(text, version, &["Marker"]), "{text}");
            assert!(project(text, version, &["Other"]), "{text}");
        }
    }
}

#[test]
fn pruned_values_bodies_and_trivia_do_not_require_complete_facts() {
    for text in [
        "class C { void method() {} }",
        "void body() { @Marker() final value = 1; void nested({int? _value}) {} }",
        "final callback = ({int? _value}) => 1;",
        "class C { static final callback = ({int? _value}) => 1; }",
        "@Other((@Marker() int value) => value) class C {}",
        "void f({Object? value = Foo(({int? _value}) => 1)}) {}",
        "const text = 'Marker @Marker _value'; // @Marker()\nclass C {}",
        "class C { void method([int? _value]) {} }",
    ] {
        for version in [
            DartLanguageVersion::DART_3_0,
            DartLanguageVersion::DART_3_12,
        ] {
            assert!(project(text, version, &["Marker"]), "{version}: {text}");
        }
    }
}

#[test]
fn private_named_parameter_gates_and_future_validation_keep_complete_facts() {
    for text in [
        "void function({int? _value}) {}",
        "class C { C({@Other() int? _value}); }",
        "class C { void method(void callback({int? _value})) {} }",
    ] {
        assert!(
            !project(text, DartLanguageVersion::DART_3_0, &["Marker"]),
            "{text}"
        );
        assert!(
            project(text, DartLanguageVersion::DART_3_12, &["Marker"]),
            "{text}"
        );
        assert!(
            !project(text, DartLanguageVersion::DART_3_13, &["Marker"]),
            "{text}"
        );
    }
}
