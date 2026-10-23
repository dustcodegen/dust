#![cfg(test)]

use dust_ir::{NameIr, RedirectTargetIr, SpanIr, TypeIr};
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::{SymbolCatalog, resolve_library};
use dust_text::{FileId, SourceText, TextRange};

#[test]
fn redirect_facts_lower_in_the_owning_binder_scope_with_original_spans() {
    let text = "/* π */ class Event<int, T> { const Event(); factory Event.done(T label) = p.Done<T, int>.create; }";
    let file = FileId::new(134);
    let source = SourceText::new(file, text);
    let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    let mut resolved = resolve_library(
        file,
        "event.dart",
        "event.g.dart",
        &parsed.library,
        &SymbolCatalog::new(),
    );
    assert_eq!(resolved.diagnostics, []);
    let outcome = super::lower_library(&mut resolved.library);
    assert_eq!(outcome.diagnostics, []);
    let class = &outcome.value.classes[0];
    let range = |fragment: &str| {
        let start = text.find(fragment).unwrap() as u32;
        SpanIr::new(file, TextRange::new(start, start + fragment.len() as u32))
    };
    assert!(class.constructors[0].is_const);
    assert!(!class.constructors[1].is_const);
    assert_eq!(class.constructors[0].redirected_target, None);
    assert_eq!(
        class.constructors[1].redirected_target,
        Some(RedirectTargetIr {
            ty: TypeIr::generic("p.Done", vec![TypeIr::named("T"), TypeIr::named("int")]),
            constructor: Some(NameIr {
                source: "create".into(),
                short: "create".into(),
                prefix: None,
                span: range("create")
            }),
            span: range("p.Done<T, int>.create"),
        })
    );
    assert_eq!(
        class.constructors[1].redirected_target_source.as_deref(),
        Some("p.Done<T, int>.create")
    );
    assert_eq!(
        class
            .type_parameters
            .iter()
            .map(|param| param.name.source.as_str())
            .collect::<Vec<_>>(),
        ["int", "T"]
    );
}
