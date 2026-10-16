//! Explicit generic function signatures and named-type classification controls.

use dust_parser_dart::{ParsedTypeKind, ParsedTypeSurface};
use dust_text::TextRange;

#[test]
fn preserves_explicit_generic_function_types_and_original_source() {
    for source in [
        "String Function<T>(T)?",
        "Function<T>(T)",
        "String Function<String extends Object>(String)",
        "(T, U) Function<T, U extends Comparable<U>>(T, U)",
        "List<T> Function<T extends Iterable<int>>(T, {void Function<U>(U)? callback})",
        "void Function<T extends Comparable<T>>(T)",
        "void Function (String)?",
    ] {
        let span = TextRange::new(37_u32, 37 + source.len() as u32);
        assert_eq!(
            ParsedTypeSurface::parse(source, span),
            Some(ParsedTypeSurface {
                source: source.to_owned(),
                span,
                kind: ParsedTypeKind::Function,
                nullable: source.ends_with('?'),
            }),
            "{source}"
        );
    }
}

#[test]
fn named_types_and_nested_signatures_are_not_outer_function_types() {
    for source in [
        "Function",
        "Function<T>",
        "FunctionAlias<T>",
        "p.Function<T>",
        "List<T Function<T>(T)>",
        "Map<String, void Function<T>(T)>",
    ] {
        let span = TextRange::new(0_u32, source.len() as u32);
        let parsed = ParsedTypeSurface::parse(source, span).unwrap();
        assert!(
            matches!(parsed.kind, ParsedTypeKind::Named { .. }),
            "{source}"
        );
    }

    let source = "List<T Function<T>(T)>";
    let parsed =
        ParsedTypeSurface::parse(source, TextRange::new(0_u32, source.len() as u32)).unwrap();
    let ParsedTypeKind::Named { args, .. } = parsed.kind else {
        panic!("expected a named outer type");
    };
    assert_eq!(args[0].kind, ParsedTypeKind::Function);
    assert_eq!(args[0].source, "T Function<T>(T)");
}

#[test]
fn records_with_generic_function_fields_remain_record_types() {
    let source = "({T Function<T>(T) transform, int count})?";
    let span = TextRange::new(0_u32, source.len() as u32);
    assert_eq!(
        ParsedTypeSurface::parse(source, span),
        Some(ParsedTypeSurface {
            source: source.to_owned(),
            span,
            kind: ParsedTypeKind::Record,
            nullable: true,
        })
    );
}
