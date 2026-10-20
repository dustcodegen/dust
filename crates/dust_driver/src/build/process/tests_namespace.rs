#![cfg(test)]

use super::*;
use dust_ir::{AnnotationIr, AnnotationOrigin, SymbolId};
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_plugin_api::PluginRegistry;
use dust_resolver::{NamespaceGraph, SymbolCatalog};
use dust_text::SourceText;
use std::path::Path;

mod failures;
mod groups;
mod trivia;

const APP: &str = "package:app/main.dart";
const CLIENT: &str = "package:dust_dart/src/http/http_client_annotations.dart";
const SERVER: &str = "package:dust_server/src/annotations/http_methods.dart";

fn parse(id: u32, source: &str) -> ParsedDartFileSurface {
    let result = TreeSitterDartBackend::new().parse_file(
        &SourceText::new(FileId::new(id), source),
        ParseOptions::default(),
    );
    assert_eq!(result.diagnostics, []);
    result.library
}

fn insert(graph: &mut NamespaceGraph, id: u32, uri: &str, parsed: &ParsedDartFileSurface) {
    graph
        .insert_library(uri, FileId::new(id), parsed, |_, value| match value {
            "main.g.dart" => Ok("package:app/main.g.dart".to_owned()),
            value if value.starts_with("package:") => Ok(value.to_owned()),
            value => Err(format!("no fixture mapping for `{value}`")),
        })
        .unwrap();
}

fn inputs(source: &str, reverse: bool) -> (NamespaceGraph, SymbolCatalog, ParsedDartFileSurface) {
    let mut graph = NamespaceGraph::new();
    insert(
        &mut graph,
        1,
        CLIENT,
        &parse(
            1,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../packages/dust_dart/lib/src/http/http_client_annotations.dart"
            )),
        ),
    );
    insert(
        &mut graph,
        2,
        SERVER,
        &parse(
            2,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../packages/dust_server/lib/src/annotations/http_methods.dart"
            )),
        ),
    );
    let parsed = parse(3, source);
    insert(&mut graph, 3, APP, &parsed);
    insert(
        &mut graph,
        4,
        "package:app/main.g.dart",
        &parse(4, &format!("part of '{APP}';")),
    );
    let mut catalog = SymbolCatalog::new();
    let mut owners = [("dust_dart::GET", CLIENT), ("dust_server::GET", SERVER)];
    if reverse {
        owners.reverse();
    }
    for (id, uri) in owners {
        catalog.register_config("GET", id);
        catalog
            .register_annotation_origin(AnnotationOrigin::new(id, uri, "GET"))
            .unwrap();
    }
    (graph, catalog, parsed)
}

fn lower(source: &str, reverse: bool) -> (Option<DartFileIr>, Vec<Diagnostic>) {
    let (graph, catalog, parsed) = inputs(source, reverse);
    pipeline(&graph, &catalog, &parsed)
}

fn pipeline(
    graph: &NamespaceGraph,
    catalog: &SymbolCatalog,
    parsed: &ParsedDartFileSurface,
) -> (Option<DartFileIr>, Vec<Diagnostic>) {
    let registry = PluginRegistry::new();
    let lowering = LoweringConfig {
        package_root: Path::new("/workspace"),
        package_name: "app",
        catalog,
        registry: &registry,
    };
    let library = SourceLibrary {
        source_path: "/workspace/lib/main.dart".into(),
        output_path: "/workspace/lib/main.g.dart".into(),
    };
    let mut diagnostics = Vec::new();
    let ir = super::execute::resolve_and_lower_library(
        FileId::new(3),
        &library,
        parsed,
        &lowering,
        dust_resolver::BindingContext::new(catalog, Some(graph.scope(APP))),
        &mut diagnostics,
    );
    (ir, diagnostics)
}

fn assert_server(annotation: &AnnotationIr, path: &str, source: &str) {
    let written = format!("@s.GET('{path}')");
    let start = source.find(&written).unwrap() as u32;
    assert_eq!(
        *annotation,
        AnnotationIr {
            raw_name: "s.GET".to_owned(),
            short_name: "GET".to_owned(),
            prefix: Some("s".to_owned()),
            positional_args: vec![dust_ir::AnnotationValueIr::String(path.to_owned())],
            named_args: std::collections::BTreeMap::new(),
            resolved_symbol: Some(SymbolId::new("dust_server::GET")),
            span: dust_ir::SpanIr::new(
                FileId::new(3),
                dust_text::TextRange::new(start, start + written.len() as u32)
            ),
        }
    );
}

#[test]
fn all_raw_metadata_sites_use_the_same_namespace_before_driver_ir() {
    let source = format!("@s.GET('/library') library app; import '{CLIENT}' as c; import '{SERVER}' as s; part 'main.g.dart';
@s.GET('/mixin') mixin M {{}}
@s.GET('/extension') extension E on String {{}}
@s.GET('/type') extension type W(int value) {{}}
@s.GET('/function') void handler(@c.GET('/parameter') int value) {{}}
@s.GET('/variable') final int value=0;
@s.GET('/typedef') typedef Alias=int;
class Api {{ @c.GET('/client') void client() {{}} @s.GET('/server') void server() {{}} }}");
    for reverse in [false, true] {
        let (ir, diagnostics) = lower(&source, reverse);
        assert_eq!(diagnostics, []);
        let ir = ir.unwrap();
        for (annotation, path) in [
            (&ir.library_annotations[0], "/library"),
            (&ir.mixins[0].annotations[0], "/mixin"),
            (&ir.extensions[0].annotations[0], "/extension"),
            (&ir.extension_types[0].annotations[0], "/type"),
            (&ir.functions[0].annotations[0], "/function"),
            (&ir.variables[0].annotations[0], "/variable"),
            (&ir.typedefs[0].annotations[0], "/typedef"),
        ] {
            assert_server(annotation, path, &source);
        }
        assert_eq!(
            ir.functions[0].params[0]
                .configs
                .iter()
                .map(|config| config.symbol.0.as_str())
                .collect::<Vec<_>>(),
            ["dust_dart::GET"]
        );
        let methods = &ir.classes[0].methods;
        assert_eq!(
            methods[0].configs[0].symbol,
            SymbolId::new("dust_dart::GET")
        );
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

#[test]
fn constructor_parameter_and_mixin_field_errors_are_not_lost_on_unannotated_types() {
    for declaration in [
        "class Plain { Plain(@s.GET('/parameter') int value); }",
        "mixin Plain { @s.GET('/field') final int value=0; }",
        "void handler(@s.GET('/parameter') int value) {}",
    ] {
        let source = format!("import '{SERVER}' as s hide GET; {declaration}");
        let (ir, diagnostics) = lower(&source, false);
        assert_eq!(ir, None, "{declaration}");
        assert_eq!(diagnostics.len(), 1, "{declaration}: {diagnostics:?}");
        assert_eq!(
            diagnostics[0].message,
            "annotation name `GET` is not visible in this namespace"
        );
        assert_eq!(diagnostics[0].labels[0].file_id, FileId::new(3));
    }
}
