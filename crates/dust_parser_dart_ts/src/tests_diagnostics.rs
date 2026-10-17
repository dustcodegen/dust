use super::*;
use dust_text::FileId;

#[test]
fn native_parse_failure_is_fatal_in_both_views_without_losing_parse_options() {
    let source = SourceText::new(FileId::new(142), "class Model {}");
    let options = ParseOptions {
        language_version: dust_dart_syntax::DartLanguageVersion::new(3, 6),
        ..Default::default()
    };
    let result = extract_parse_result(None, &source, options);
    assert_eq!(
        result.diagnostics,
        [Diagnostic::error("tree-sitter failed to parse source")]
    );
    assert_eq!(result.namespace_diagnostics, result.diagnostics);
    assert!(result.has_errors());
    assert!(result.library.is_empty());
    assert_eq!(result.library.span, source.full_range());
    assert_eq!(result.options, options);
}
