use crate::support::parse;

fn modifier_diagnostic_count(result: &dust_parser_dart::ParseResult, keyword: &str) -> usize {
    let message = format!("`{keyword}` is only valid on primary-constructor declaring parameters");
    result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.message.contains(&message))
        .count()
}

#[test]
fn reports_diagnostic_for_malformed_dart_source() {
    let result = parse(
        3,
        r#"
class Broken {
  final String name
}
"#,
    );

    assert!(result.has_errors());
    assert!(!result.diagnostics.is_empty());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("syntax error"))
    );
}

#[test]
fn explains_syntax_errors_without_claiming_the_dart_source_is_invalid() {
    for source in [
        "class Broken { final String name }",
        "void broken() { final value = ; }",
    ] {
        let result = parse(313, source);
        let syntax_errors = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.message == "tree-sitter reported a Dart syntax error")
            .collect::<Vec<_>>();
        assert!(
            !syntax_errors.is_empty(),
            "{source}: {:?}",
            result.diagnostics
        );
        assert_eq!(
            syntax_errors[0].notes,
            vec![
                "Check this file with `dart analyze`. Valid Dart may use syntax not supported by Dust's bundled parser.",
                "Named concise `new` constructors are not supported yet. Use an in-body constructor with the class name.",
            ]
        );
        assert!(syntax_errors[0].has_labels());
        assert!(
            syntax_errors
                .iter()
                .skip(1)
                .all(|diagnostic| diagnostic.notes.is_empty())
        );
    }
}

#[test]
fn preserves_preorder_for_missing_nodes_and_other_diagnostics() {
    let source = "// @dart=3.13\nclass Broken { final String name }\nfinal status = .ready;\nvoid broken() { final value = ; }";
    let result = parse(404, source);
    let summaries = result
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let label = &diagnostic.labels[0];
            assert_eq!(label.file_id, dust_text::FileId::new(404));
            (
                diagnostic.message.as_str(),
                label.range.start().to_usize()..label.range.end().to_usize(),
                diagnostic.notes.len(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        summaries,
        [
            ("tree-sitter reported a Dart syntax error", 46..46, 2),
            ("Dust does not support dot shorthands yet", 64..70, 2),
            ("tree-sitter reported a Dart syntax error", 100..101, 0),
        ]
    );
    assert_eq!(result.namespace_diagnostics, result.diagnostics);
}

#[test]
fn does_not_add_unsupported_syntax_guidance_to_supported_source() {
    let result = parse(
        314,
        r#"class User {
  const User(this.name);
  final String name;
}
"#,
    );
    assert_eq!(result.diagnostics, Vec::new());
}

#[test]
fn reports_invalid_parameter_modifiers_for_dart_3_13_normal_params() {
    let result = parse(
        310,
        r#"
// @dart=3.13
void saveFinal(final String name) {}
void saveVar(var String name) {}

class User {
  const User(final String id);

  void update({
    required final String name,
    covariant var int count,
  }) {}
}
"#,
    );

    assert!(result.has_errors());
    assert_eq!(modifier_diagnostic_count(&result, "final"), 3);
    assert_eq!(modifier_diagnostic_count(&result, "var"), 2);
    assert!(
        result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .notes
                .iter()
                .any(|note| note.contains("Use `Type name` for normal function"))
        }),
        "diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn preserves_normal_parameter_modifiers_before_dart_3_13() {
    let result = parse(
        311,
        r#"
// @dart=3.12
void saveFinal(final String name) {}
void saveVar(var String name) {}

class User {
  const User(final String id);
  void update({required final String name, var int count = 0}) {}
}
"#,
    );

    assert_eq!(modifier_diagnostic_count(&result, "final"), 0);
    assert_eq!(modifier_diagnostic_count(&result, "var"), 0);
}

#[test]
fn ignores_existing_field_and_super_formals() {
    let result = parse(
        312,
        r#"
// @dart=3.13
class Parent {
  const Parent({Object? key});
}

class User extends Parent {
  const User(this.id, {required this.name, super.key});

  final String id;
  final String name;
}
"#,
    );

    assert_eq!(modifier_diagnostic_count(&result, "final"), 0);
    assert_eq!(modifier_diagnostic_count(&result, "var"), 0);
}
