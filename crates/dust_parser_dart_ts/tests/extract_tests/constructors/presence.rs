use crate::support::parse;
use dust_parser_dart::ParameterKind::{Named, Positional};

#[test]
fn constructor_presence_is_independent_of_nullability_and_defaults() {
    for source in [
        "class Shape { Shape(int? first, [int? second, int third = 3]); Shape.named({required int? fourth, int? fifth, int sixth = 6}); }",
        "class Shape(int? first, [int? second, int third = 3]) { Shape.named({required int? fourth, int? fifth, int sixth = 6}); }",
        "class Shape { Shape(this.first, [this.second, this.third = 3]); Shape.named({required this.fourth, this.fifth, this.sixth = 6}); }",
        "class Shape { Shape(super.first, [super.second, super.third = 3]); Shape.named({required super.fourth, super.fifth, super.sixth = 6}); }",
        "sealed class Shape { const factory Shape(int? first, [int? second, int? third]) = Value; const factory Shape.named({required int? fourth, int? fifth, int? sixth}) = NamedValue; }",
    ] {
        let result = parse(138, source);
        assert_eq!(result.diagnostics, [], "{source}");
        let params = result.library.classes[0]
            .constructors
            .iter()
            .flat_map(|constructor| &constructor.params)
            .map(|param| (param.name.as_str(), param.kind, param.is_required))
            .collect::<Vec<_>>();
        assert_eq!(
            params,
            [
                ("first", Positional, true),
                ("second", Positional, false),
                ("third", Positional, false),
                ("fourth", Named, true),
                ("fifth", Named, false),
                ("sixth", Named, false),
            ],
            "{source}"
        );
    }
}

#[test]
fn required_words_inside_annotations_comments_and_callbacks_are_not_modifiers() {
    let source = r#"
class Shape {
  Shape({
    @Tag('required') int? annotated,
    /* required */ int? commented,
    void Function({required int value})? callback,
    required /* trivia */ int? actual,
    int? next,
  });
  void method({@Tag('required') int? annotated, /* required */ int? commented,
    void Function({required int value})? callback, required int? actual, int? next}) {}
}
"#;
    let result = parse(138, source);
    assert_eq!(result.diagnostics, []);
    let class = &result.library.classes[0];
    assert_eq!(
        class.constructors[0]
            .params
            .iter()
            .map(|param| (param.name.as_str(), param.is_required))
            .collect::<Vec<_>>(),
        [
            ("annotated", false),
            ("commented", false),
            ("callback", false),
            ("actual", true),
            ("next", false)
        ]
    );
    assert_eq!(
        class.methods[0]
            .params
            .iter()
            .map(|param| (param.name.as_str(), param.is_required))
            .collect::<Vec<_>>(),
        [
            ("annotated", false),
            ("commented", false),
            ("callback", false),
            ("actual", true),
            ("next", false)
        ]
    );
}
