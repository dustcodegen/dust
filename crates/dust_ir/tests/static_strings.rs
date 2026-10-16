//! Static values must not be reconstructed from rejected annotation expressions.

use dust_ir::{AnnotationValueIr, ConfigApplicationIr, ExprSourceIr, SpanIr, SymbolId};
use dust_text::{FileId, TextRange};

/// Raw-only callers exercise the compatibility path without parser facts.
fn config(source: &str) -> ConfigApplicationIr {
    ConfigApplicationIr::new(
        SymbolId::new("dust_dart::Query"),
        Some(source.to_owned()),
        SpanIr::new(FileId::new(1), TextRange::new(0_u32, 1_u32)),
    )
}

#[test]
fn compatibility_accessors_use_the_shared_static_decoder() {
    let valid = config(r"('\u0053ELECT \x31', sql: '\u0053ELECT \u{31}')");
    assert_eq!(valid.positional_string(0).as_deref(), Some("SELECT 1"));
    assert_eq!(valid.named_string("sql").as_deref(), Some("SELECT 1"));
    let invalid = config(r"('\uD800', sql: '$table')");
    assert_eq!(invalid.positional_string(0), None);
    assert_eq!(invalid.named_string("sql"), None);
}

#[test]
fn structured_nonstring_values_never_fall_back_to_raw_text() {
    let mut config = config("('looks static', sql: 'looks static')");
    let expression = AnnotationValueIr::Expression(ExprSourceIr {
        source: "'looks static'".to_owned(),
        span: config.span,
    });
    config.positional_args.push(expression.clone());
    config.named_args.insert("sql".to_owned(), expression);
    assert_eq!(config.positional_string(0), None);
    assert_eq!(config.named_string("sql"), None);
    config.positional_args[0] = AnnotationValueIr::String("semantic".to_owned());
    config.named_args.insert(
        "sql".to_owned(),
        AnnotationValueIr::String("semantic".to_owned()),
    );
    assert_eq!(config.positional_string(0).as_deref(), Some("semantic"));
    assert_eq!(config.named_string("sql").as_deref(), Some("semantic"));
}
