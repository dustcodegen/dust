use dust_parser_dart::DartLanguageVersion;
use dust_text::FileId;

use super::*;

#[test]
fn empty_placeholders_match_native_parsing_for_both_extraction_modes() {
    let backend = TreeSitterDartBackend::new();
    for id in [FileId::new(0), FileId::new(137)] {
        let source = SourceText::new(id, "");
        for version in [
            DartLanguageVersion::DART_3_0,
            DartLanguageVersion::DART_3_13,
        ] {
            let options = ParseOptions {
                language_version: version,
                ..Default::default()
            };
            let tree = PARSER.with(|parser| parser.borrow_mut().parse(source.as_str(), None));
            let expected = extract_parse_result(tree.clone(), &source, options);
            assert_eq!(backend.parse_file(&source, options), expected);
            assert_eq!(
                backend.parse_namespace_file(&source, options, &["Derive"]),
                extract_parse_result_with_projection(tree, &source, options, Some(&["Derive"])),
            );
            assert_eq!(expected.library.lexical_scopes.len(), 1);
            assert_eq!(
                expected.library.lexical_scopes[0].kind,
                ParsedLexicalScopeKind::Library
            );
            assert_eq!(expected.options, options);
        }
    }
}

#[test]
fn whitespace_and_comments_continue_using_the_original_native_surface() {
    let backend = TreeSitterDartBackend::new();
    let options = ParseOptions::default();
    for text in [" ", "\n\t ", "// comment\n", "// @dart=3.0\n"] {
        let source = SourceText::new(FileId::new(14), text);
        let tree = PARSER.with(|parser| parser.borrow_mut().parse(text, None));
        assert_eq!(
            backend.parse_file(&source, options),
            extract_parse_result(tree.clone(), &source, options)
        );
        assert_eq!(
            backend.parse_namespace_file(&source, options, &[]),
            extract_parse_result_with_projection(tree, &source, options, Some(&[])),
        );
    }
}

#[test]
fn native_parse_failure_retains_its_distinct_empty_scope_contract() {
    let source = SourceText::new(FileId::new(19), "");
    let failed = extract_parse_result(None, &source, ParseOptions::default());
    assert!(failed.has_errors());
    assert!(failed.library.lexical_scopes.is_empty());
}
