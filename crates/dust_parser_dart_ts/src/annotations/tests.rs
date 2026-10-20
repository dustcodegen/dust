use dust_parser_dart::{ParseBackend, ParseOptions, ParsedAnnotationValueRootKind};

use super::*;

#[test]
fn annotation_identifiers_exclude_trivia_without_rewriting_source() {
    for written in [
        "c . GET",
        "c /* owner . ( */ . GET",
        "c . /* owner ) . */ GET",
        "c // owner . (\n . GET",
        "c /* outer /* nested . */ inner */ . GET",
    ] {
        let text = format!("@{written}('/path') class C {{}}");
        let source = SourceText::new(dust_text::FileId::new(0), text.as_str());
        let result =
            crate::TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
        assert_eq!(result.diagnostics, []);
        let annotation = &result.library.classes[0].annotations[0];
        assert_eq!(annotation.name, "GET");
        assert_eq!(annotation.prefix.as_deref(), Some("c"), "{written}");
        assert_eq!(annotation.qualified_name, written);
        assert_eq!(annotation.arguments_source.as_deref(), Some("('/path')"));
        assert_eq!(
            annotation.span,
            TextRange::new(0_u32, text.find(" class").unwrap() as u32)
        );
        let arguments = annotation.parsed_arguments.as_ref().unwrap();
        assert_eq!(arguments.named, []);
        assert_eq!(arguments.positional.len(), 1);
        assert_eq!(arguments.positional[0].source, "'/path'");
        assert_eq!(
            arguments.positional[0].value.as_ref().unwrap().kind,
            ParsedAnnotationValueRootKind::String("/path".to_owned())
        );
    }
}

#[test]
fn annotation_trivia_keeps_every_existing_qualified_name_segment() {
    let written = "p /* outer */ . Marker /* constructor */ . named";
    let text = format!("@{written}() class C {{}}");
    let source = SourceText::new(dust_text::FileId::new(0), text.as_str());
    let result = crate::TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(result.diagnostics, []);
    let annotation = &result.library.classes[0].annotations[0];
    assert_eq!(annotation.name, "named");
    assert_eq!(annotation.prefix.as_deref(), Some("p.Marker"));
    assert_eq!(annotation.qualified_name, written);
}
