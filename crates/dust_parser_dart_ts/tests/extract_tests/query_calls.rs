use dust_parser_dart::ParsedQueryFunction;

use crate::support::parse;

#[test]
fn generic_query_without_optional_parameters_keeps_complete_facts() {
    let source = r#"
void load() {
  queryScalar<int>(r"SELECT COUNT(*) FROM users").fetchOne(tx);
}
"#;
    let result = parse(74, source);
    assert_eq!(result.diagnostics, []);
    let [call] = result.library.query_calls.as_slice() else {
        panic!("one query expected: {:#?}", result.library.query_calls);
    };
    assert_eq!(call.function, ParsedQueryFunction::Scalar);
    assert_eq!(call.type_arg_source.as_deref(), Some("int"));
    assert_eq!(call.sql, "SELECT COUNT(*) FROM users");
    assert!(call.sql_source_static);
    assert_eq!(call.parameter_count, 0);
    assert!(call.params_source_is_list);
    assert_eq!(call.fetch_method.as_deref(), Some("fetchOne"));
    assert_eq!(
        &source[call.span.start().to_usize()..call.span.end().to_usize()],
        r#"queryScalar<int>(r"SELECT COUNT(*) FROM users")"#
    );
}
