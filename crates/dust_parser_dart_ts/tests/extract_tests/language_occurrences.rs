use crate::support::parse;
use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_text::{FileId, TextRange};

fn gated(source: &str) -> Vec<Diagnostic> {
    parse(115, source)
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.message.contains("require Dart"))
        .collect()
}

#[test]
fn comments_and_string_contents_are_not_language_features() {
    let source = r#"// @dart=2.19
// final (x,y) = (1,2); case (x,y): interface class C {}
/// [...? values], [?.red], return .ready; private named args are not code.
/* = .ready; base class C {} */
final plain = '= (1, 2); case (x,y); final class C {}';
final raw = r'(?:[.,](\d+))? ...? [. const .ready';
final triple = '''
  ?value
  return .ready;
  mixin class C {}
''';
void main() {}
"#;
    assert_eq!(parse(115, source).diagnostics, []);
}

#[test]
fn ordinary_parentheses_and_legacy_spreads_are_not_new_features() {
    for body in [
        "final n = (1 + 2);",
        "List<int>? values; final items = [...?values];",
        "List<int>? values; final items = [if (true) ...?values];",
        "void f(Object value) { switch (value) { case 1: break; default: break; } }",
    ] {
        let source = format!("// @dart=2.19\n{body}");
        assert_eq!(parse(115, &source).diagnostics, [], "{body}");
    }
}

#[test]
fn real_inline_and_nested_null_aware_elements_have_exact_gate_spans() {
    for body in [
        "int? value; final items = [?value];",
        "int? value; final items = [if (true) ?value];",
        "int? value; final items = [for (final x in [1]) ?value];",
        "int? value; final items = {?value: 1};",
        "int? value; final items = {1: ?value};",
    ] {
        let source = format!("// @dart=3.6\n{body}");
        let offset = source.rfind("?value").unwrap() as u32;
        assert_eq!(gated(&source), [Diagnostic::error("null-aware collection elements require Dart 3.8 or newer; current language version is 3.6").with_label(SourceLabel::new(FileId::new(115), TextRange::new(offset, offset+1), "requires Dart 3.8"))], "{body}");
        assert_eq!(
            parse(115, &source.replace("@dart=3.6", "@dart=3.8")).diagnostics,
            [],
            "{body}"
        );
    }
}

#[test]
fn interpolation_code_is_checked_even_when_string_text_is_not() {
    let source = "// @dart=3.6\nint? value; final text = '${[?value]}';";
    assert_eq!(gated(source).len(), 1);
    assert_eq!(
        gated(source)[0].message,
        "null-aware collection elements require Dart 3.8 or newer; current language version is 3.6"
    );
}

#[test]
fn actual_records_patterns_and_class_modifiers_are_still_gated() {
    for (body, expected) in [
        ("final pair = (1, 2);", "records"),
        ("typedef Pair = (int, String);", "records"),
        ("void f(Object x) { if (x case int value) {} }", "patterns"),
        ("void f(Object x) { if (x case 1) {} }", "patterns"),
        (
            "void f(Object x) { final values = [if (x case 1) 2]; }",
            "patterns",
        ),
        (
            "void f(Object x) { switch (x) { case int value: break; } }",
            "patterns",
        ),
        ("final class C {}", "class modifiers"),
        ("base mixin M {}", "class modifiers"),
        ("mixin class C {}", "class modifiers"),
    ] {
        let source = format!("// @dart=2.19\n{body}");
        assert_eq!(
            gated(&source)
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>(),
            [format!(
                "{expected} require Dart 3.0 or newer; current language version is 2.19"
            )],
            "{body}"
        );
    }
}

#[test]
fn dot_shorthand_is_detected_without_whitespace_needles_but_not_on_generic_calls() {
    let source = "// @dart=3.6\nenum Kind { ready } final Kind k=.ready;";
    let offset = source.find(".ready").unwrap() as u32;
    assert_eq!(
        gated(source),
        [Diagnostic::error(
            "dot shorthands require Dart 3.10 or newer; current language version is 3.6"
        )
        .with_label(SourceLabel::new(
            FileId::new(115),
            TextRange::new(offset, offset + 1),
            "requires Dart 3.10"
        ))]
    );
    assert_eq!(
        parse(
            115,
            "// @dart=3.6\nclass Box<T> { static int value() => 1; } final n=Box<int>.value();"
        )
        .diagnostics,
        []
    );
}

#[test]
fn exact_byte_offsets_survive_unicode_comments_and_crlf() {
    let source = "// @dart=3.6\r\n// λ ...? = .ignored\r\nint? value; final items=[?value];";
    let offset = source.rfind("?value").unwrap() as u32;
    assert_eq!(
        gated(source)[0].labels[0].range,
        TextRange::new(offset, offset + 1)
    );
}
