use dust_dart_emit::dart_string_literal;
use dust_ir::AnnotationValueIr;

/// Renders a structured annotation constant without falling back to source text.
pub(super) fn render_const(value: &AnnotationValueIr, explicit_const: bool) -> String {
    match value {
        AnnotationValueIr::Null => "null".to_owned(),
        AnnotationValueIr::Bool(value) => value.to_string(),
        AnnotationValueIr::String(value) => dart_string_literal(value),
        AnnotationValueIr::Number { source, .. } => source.clone(),
        AnnotationValueIr::List(values) => render_values('[', ']', values),
        AnnotationValueIr::Set(values) => render_values('{', '}', values),
        AnnotationValueIr::Map(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!(
                    "{}: {}",
                    render_const(key, false),
                    render_const(value, false)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        AnnotationValueIr::Record(values) => format!(
            "({})",
            values
                .iter()
                .map(|(name, value)| format!("{name}: {}", render_const(value, false)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        AnnotationValueIr::Constructor {
            name,
            positional_args,
            named_args,
        } => {
            let arguments = positional_args
                .iter()
                .map(|value| render_const(value, false))
                .chain(
                    named_args
                        .iter()
                        .map(|(name, value)| format!("{name}: {}", render_const(value, false))),
                )
                .collect::<Vec<_>>()
                .join(", ");
            let prefix = if explicit_const { "const " } else { "" };
            format!("{prefix}{}({arguments})", name.source)
        }
        AnnotationValueIr::Member(name) => name.source.clone(),
        AnnotationValueIr::Expression(_) => {
            unreachable!("opaque middleware expressions are rejected during planning")
        }
    }
}

/// Renders a constant collection with deterministic separators.
fn render_values(open: char, close: char, values: &[AnnotationValueIr]) -> String {
    format!(
        "{open}{}{close}",
        values
            .iter()
            .map(|value| render_const(value, false))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dust_ir::{AnnotationNumberKindIr, ExprSourceIr, NameIr, SpanIr};
    use dust_text::{FileId, TextRange};

    use super::*;

    fn span() -> SpanIr {
        SpanIr::new(FileId::new(1), TextRange::new(0_u32, 1_u32))
    }

    fn name(source: &str) -> NameIr {
        NameIr {
            source: source.to_owned(),
            short: source.rsplit('.').next().unwrap_or(source).to_owned(),
            prefix: source.split_once('.').map(|(prefix, _)| prefix.to_owned()),
            span: span(),
        }
    }

    #[test]
    fn renders_every_structured_constant_shape() {
        let value = AnnotationValueIr::Constructor {
            name: name("layers.TraceLayer"),
            positional_args: vec![AnnotationValueIr::String("root's".to_owned())],
            named_args: BTreeMap::from([
                ("enabled".to_owned(), AnnotationValueIr::Bool(true)),
                (
                    "options".to_owned(),
                    AnnotationValueIr::Map(vec![(
                        AnnotationValueIr::String("limit".to_owned()),
                        AnnotationValueIr::Number {
                            source: "2".to_owned(),
                            kind: AnnotationNumberKindIr::Int,
                        },
                    )]),
                ),
                (
                    "tags".to_owned(),
                    AnnotationValueIr::List(vec![AnnotationValueIr::Member(name("tag"))]),
                ),
            ]),
        };

        assert_eq!(
            render_const(&value, true),
            "const layers.TraceLayer('root\\'s', enabled: true, options: {'limit': 2}, tags: [tag])"
        );
        assert_eq!(
            render_const(
                &AnnotationValueIr::Set(vec![AnnotationValueIr::Null]),
                false
            ),
            "{null}"
        );
        assert_eq!(
            render_const(
                &AnnotationValueIr::Record(vec![(
                    "value".to_owned(),
                    AnnotationValueIr::String("ok".to_owned()),
                )]),
                false
            ),
            "(value: 'ok')"
        );
    }

    #[test]
    #[should_panic(expected = "opaque middleware expressions")]
    fn opaque_expressions_never_reach_emission() {
        render_const(
            &AnnotationValueIr::Expression(ExprSourceIr {
                source: "makeLayer()".to_owned(),
                span: span(),
            }),
            true,
        );
    }
}
