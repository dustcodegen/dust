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
        "class const User(final String name);",
        "class User<T>(final T value);",
        "class User { const new create(); }",
        "class Broken { final String name }",
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
                "Primary constructors and named concise `new` constructors are not supported yet. Use traditional in-body constructors with the class name and explicit fields.",
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
