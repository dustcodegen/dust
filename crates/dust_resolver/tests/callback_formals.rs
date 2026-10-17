//! Source-preserving callback lowering and explicit failure boundaries.

use dust_ir::TypeIr;
use dust_parser_dart::{
    ParseBackend, ParseOptions, ParsedFunctionFormalTypeSurface, ParsedTypeKind,
};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::lower_parameter_type_ir;
use dust_text::{FileId, SourceText};

fn formal(parameter: &str) -> ParsedFunctionFormalTypeSurface {
    let source = SourceText::new(FileId::new(42), format!("void consume({parameter}) {{}}"));
    let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    parsed.library.functions[0].params[0]
        .function_formal
        .clone()
        .unwrap()
}

#[test]
fn lowers_canonical_callables_without_mutating_original_facts() {
    for (source, signature, nullable) in [
        ("void callback()", "void Function()", false),
        (
            "bool? callback(String value)",
            "bool? Function(String value)",
            false,
        ),
        (
            "bool callback(String value)?",
            "bool Function(String value)",
            true,
        ),
        (
            "T callback<T extends Object, U>(T value, [U? other])",
            "T Function<T extends Object, U>(T value, [U? other])",
            false,
        ),
        (
            "bool callback(bool nested(String value)?)",
            "bool Function(bool Function(String value)? nested)",
            false,
        ),
        (
            "bool callback({required String value, int? count})",
            "bool Function({required String value, int? count})",
            false,
        ),
        (
            "p.Result callback(List<p.Value?> values)",
            "p.Result Function(List<p.Value?> values)",
            false,
        ),
        (
            "bool callback(bool Function(String) nested)",
            "bool Function(bool Function(String) nested)",
            false,
        ),
    ] {
        let formal = formal(source);
        let original = formal.clone();
        let outcome = lower_parameter_type_ir(
            FileId::new(42),
            formal.return_type.as_ref(),
            Some("ignored"),
            Some(&formal),
            &["String"],
        );
        assert_eq!(outcome.diagnostics, [], "{source}");
        let ty = TypeIr::function(signature);
        assert_eq!(outcome.value, if nullable { ty.nullable() } else { ty });
        assert_eq!(formal, original);
    }
}

#[test]
fn inference_unknown_types_and_signature_defaults_fail_with_original_labels() {
    let mut missing_return = formal("bool callback(String value)");
    missing_return.return_type = None;
    let mut missing_name = formal("bool callback(String value)");
    missing_name.parameters[0].name.name.clear();
    let mut missing_argument = formal("bool callback(String value)");
    missing_argument.parameters[0].parsed_type = None;
    let mut unknown_return = formal("bool callback(String value)");
    unknown_return.return_type.as_mut().unwrap().kind = ParsedTypeKind::Unknown;
    let mut unknown_nested_type = formal("bool callback(List<String> value)");
    let ParsedTypeKind::Named { args, .. } = &mut unknown_nested_type.parameters[0]
        .parsed_type
        .as_mut()
        .unwrap()
        .kind
    else {
        panic!()
    };
    args[0].kind = ParsedTypeKind::Unknown;
    let mut unknown_bound = formal("T callback<T extends Object>(T value)");
    unknown_bound.type_parameters[0]
        .bound
        .as_mut()
        .unwrap()
        .kind = ParsedTypeKind::Unknown;
    for formal in [
        missing_return,
        missing_name,
        missing_argument,
        unknown_return,
        unknown_nested_type,
        unknown_bound,
        formal("bool callback([String value = 'yes'])"),
        formal("bool callback(bool nested([String value = 'yes']))"),
    ] {
        let outcome = lower_parameter_type_ir(
            FileId::new(42),
            formal.return_type.as_ref(),
            Some("bool"),
            Some(&formal),
            &[],
        );
        assert_eq!(outcome.value, TypeIr::Unknown);
        assert_eq!(outcome.diagnostics.len(), 1);
        let diagnostic = &outcome.diagnostics[0];
        assert!(diagnostic.is_error());
        assert_eq!(diagnostic.message, "Cannot lower declaring callback type");
        assert_eq!(diagnostic.labels.len(), 1);
        assert_eq!(diagnostic.labels[0].file_id, FileId::new(42));
        assert!(diagnostic.labels[0].range.start() >= formal.span.start());
        assert!(diagnostic.labels[0].range.end() <= formal.span.end());
    }
}

#[test]
fn ordinary_parameter_lowering_keeps_its_existing_scope_and_fallback_policy() {
    assert_eq!(
        lower_parameter_type_ir(FileId::new(42), None, Some("String?"), None, &["String"]).value,
        TypeIr::named("String").nullable()
    );
    assert_eq!(
        lower_parameter_type_ir(FileId::new(42), None, None, None, &[]).value,
        TypeIr::Unknown
    );
}
