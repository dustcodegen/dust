#![cfg(test)]

use super::*;

#[test]
fn outer_annotation_trivia_binds_before_normalization_and_raw_lowering() {
    for separator in [
        " . ",
        " /* owner . ( */ . ",
        " . /* owner ) . */ ",
        " // owner . (\n . ",
    ] {
        let client = format!("c{separator}GET");
        let server = format!("s{separator}GET");
        let source = format!("@{server}('/library') library app; import '{CLIENT}' as c; import '{SERVER}' as s; part 'main.g.dart';
@{server}('/function') void handler(@{client}('/parameter') int value) {{}}
class Api {{ @{client}('/client') void client() {{}} @{server}('/server') void server() {{}} }}");
        for reverse in [false, true] {
            let (ir, diagnostics) = lower(&source, reverse);
            assert_eq!(diagnostics, [], "{separator}");
            let ir = ir.unwrap();
            for (annotation, path) in [
                (&ir.library_annotations[0], "/library"),
                (&ir.functions[0].annotations[0], "/function"),
            ] {
                let written = format!("@{server}('{path}')");
                let start = source.find(&written).unwrap() as u32;
                assert_eq!(
                    *annotation,
                    AnnotationIr {
                        raw_name: server.clone(),
                        short_name: "GET".to_owned(),
                        prefix: Some("s".to_owned()),
                        positional_args: vec![dust_ir::AnnotationValueIr::String(path.to_owned())],
                        named_args: Default::default(),
                        resolved_symbol: Some(SymbolId::new("dust_server::GET")),
                        span: dust_ir::SpanIr::new(
                            FileId::new(3),
                            dust_text::TextRange::new(start, start + written.len() as u32)
                        )
                    }
                );
            }
            assert_eq!(
                ir.functions[0].params[0].configs[0].symbol,
                SymbolId::new("dust_dart::GET")
            );
            let methods = &ir.classes[0].methods;
            assert_eq!(
                methods[0].configs[0].normalized,
                Some(dust_ir::NormalizedConfigIr::Http(
                    dust_ir::HttpConfigIr::Verb {
                        verb: dust_ir::HttpVerbIr::Get,
                        path: "/client".to_owned()
                    }
                ))
            );
            assert_eq!(
                methods[1].configs[0].symbol,
                SymbolId::new("dust_server::GET")
            );
            assert_eq!(methods[1].configs[0].normalized, None);
        }
    }
}

#[test]
fn annotation_trivia_cannot_bypass_visibility_or_lose_raw_failure_labels() {
    for (declaration, hide) in [
        (
            "@s /* owner . ( */ . GET('/method') class Broken {}",
            "hide GET",
        ),
        (
            "@s /* owner . ( */ . GET('/function') void handler() {}",
            "hide GET",
        ),
        (
            "@missing /* owner . ( */ . GET('/function') void handler() {}",
            "",
        ),
    ] {
        let source = format!("import '{SERVER}' as s {hide}; {declaration}");
        let (ir, diagnostics) = lower(&source, false);
        assert_eq!(ir, None);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].message,
            if hide.is_empty() {
                "annotation prefix `missing` is not declared by an import"
            } else {
                "annotation name `GET` is not visible in this namespace"
            }
        );
        let label = &diagnostics[0].labels[0];
        assert_eq!(label.file_id, FileId::new(3));
        assert_eq!(
            label.message,
            if hide.is_empty() {
                "cannot bind annotation `missing /* owner . ( */ . GET`"
            } else {
                "cannot bind annotation `s /* owner . ( */ . GET`"
            }
        );
        assert_eq!(
            &source[label.range.start().to_usize()..label.range.end().to_usize()],
            declaration
                .split_once(" class")
                .or_else(|| declaration.split_once(" void"))
                .unwrap()
                .0
        );
    }
}

#[test]
fn context_free_prefix_validation_uses_identifiers_and_keeps_unique_legacy_lookup() {
    let written = "c /* owner . ( */ . GET";
    let source = format!(
        "import '{CLIENT}' as c; part 'main.g.dart'; class Api {{ @{written}('/client') void client() {{}} }}"
    );
    let parsed = parse(3, &source);
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("GET", "dust_dart::GET");
    let result = dust_resolver::resolve_library(
        FileId::new(3),
        "lib/main.dart",
        "lib/main.g.dart",
        &parsed,
        &catalog,
    );
    assert_eq!(result.diagnostics, []);
    let method = &result.library.classes[0].methods[0];
    assert_eq!(method.configs[0].symbol, SymbolId::new("dust_dart::GET"));
    assert_eq!(
        method.configs[0].normalized,
        Some(dust_ir::NormalizedConfigIr::Http(
            dust_ir::HttpConfigIr::Verb {
                verb: dust_ir::HttpVerbIr::Get,
                path: "/client".to_owned()
            }
        ))
    );
    let metadata = dust_resolver::resolve_annotation_ir(
        FileId::new(3),
        &method.surface.annotations[0],
        &catalog,
    );
    assert_eq!(metadata.raw_name, written);
    assert_eq!(metadata.prefix.as_deref(), Some("c"));
    assert_eq!(
        metadata.resolved_symbol,
        Some(SymbolId::new("dust_dart::GET"))
    );
}
