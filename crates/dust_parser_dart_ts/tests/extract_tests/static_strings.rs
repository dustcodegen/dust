use dust_parser_dart::ParsedAnnotationValueRootKind as Kind;
use dust_parser_dart_ts::scan_i18n_source;
use dust_text::{FileId, SourceText};

use crate::support::parse;

#[test]
fn decoded_annotation_values_keep_exact_source_and_byte_spans() {
    for literal in [r"'\u0061\x62\u{63}'", "''' \nabc'''", r"r'abc'"] {
        let source = format!("// café\n@Meta(value: {literal}) class Model {{}}");
        let result = parse(17, &source);
        assert_eq!(result.diagnostics, vec![]);
        let argument = &result.library.classes[0].annotations[0]
            .parsed_arguments
            .as_ref()
            .unwrap()
            .named[0]
            .value
            .as_ref()
            .unwrap();
        assert_eq!(argument.kind, Kind::String("abc".to_owned()));
        assert_eq!(argument.source, literal);
        assert_eq!(
            &source[argument.span.start().to_usize()..argument.span.end().to_usize()],
            literal
        );
    }
}

#[test]
fn unsupported_annotation_literals_remain_expressions_not_guessed_strings() {
    for literal in [r"'\uD800'", r"'$table'", "'a' 'b'"] {
        let source = format!("@Meta(value: {literal}) class Model {{}}");
        let result = parse(17, &source);
        assert_eq!(result.diagnostics, vec![]);
        let argument = &result.library.classes[0].annotations[0]
            .parsed_arguments
            .as_ref()
            .unwrap()
            .named[0]
            .value
            .as_ref()
            .unwrap();
        assert_eq!(argument.kind, Kind::Expression, "{literal}");
        assert_eq!(argument.source, literal);
    }
}

#[test]
fn checked_sql_decodes_without_changing_the_call_span() {
    let call = r"queryScalar<int>('\u0053ELECT \x31', []).fetchOne(db)";
    let source = format!("// café\nvoid main() {{ {call}; }}");
    let result = parse(17, &source);
    assert_eq!(result.diagnostics, vec![]);
    assert_eq!(result.library.query_calls.len(), 1);
    let query = &result.library.query_calls[0];
    assert_eq!(query.sql, "SELECT 1");
    assert!(query.sql_source_static);
    assert_eq!(
        &source[query.span.start().to_usize()..query.span.end().to_usize()],
        r"queryScalar<int>('\u0053ELECT \x31', [])"
    );
}

#[test]
fn i18n_decodes_keys_defaults_and_placeholders_and_rejects_unpaired_surrogates() {
    let call =
        r"context.tr('\u0068ome.\x74itle', defaultText: '\u{1f680}', args: {'\u006eame': name})";
    let source = SourceText::new(
        FileId::new(17),
        format!("// café\nvoid main() {{ {call}; }}"),
    );
    let result = scan_i18n_source(&source);
    assert_eq!(result.diagnostics, vec![]);
    assert_eq!(result.entries.len(), 1);
    let entry = &result.entries[0];
    assert_eq!(entry.key, "home.title");
    assert_eq!(entry.namespace, "home");
    assert_eq!(entry.default_text.as_deref(), Some("🚀"));
    assert_eq!(entry.args, ["name"]);
    assert_eq!(source.slice(entry.span), Some(call));

    let source = SourceText::new(FileId::new(17), r"void main() { context.tr('\uD800'); }");
    let result = scan_i18n_source(&source);
    assert_eq!(result.entries, vec![]);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].message,
        "context.tr requires a string literal key"
    );
}
