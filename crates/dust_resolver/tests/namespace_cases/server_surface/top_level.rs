use super::*;
use dust_parser_dart::ParsedDirective;
use dust_resolver::resolve_annotation_ir_in_namespace;

#[test]
fn library_function_and_remaining_parameter_annotations_keep_scoped_identity() {
    let source = format!(
        r#"@s.Routes('/notes', name: 'notesRoutes')
library notes;
import '{BARREL}' as s;
@s.GET('/all')
void read(@s.Queries() Map<String, String> queries,
    @s.RawQuery() String rawQuery, @s.HeaderMap() Map<String, String> headers,
    @s.RawBody() String body, @s.State() Object state, @s.Peer() Object peer,
    @s.Extract(Extractor) Object user) {{}}
class Extractor {{ const Extractor(); }}
"#
    );
    let parsed = parse(30, &source);
    let mut graph = surface();
    add(&mut graph, 30, APP, &source);
    let ParsedDirective::Library { annotations, .. } = &parsed.directives[0] else {
        panic!("library annotation must not be lost")
    };
    assert_eq!(parsed.functions.len(), 1);
    let function = &parsed.functions[0];
    let sites = annotations
        .iter()
        .chain(&function.annotations)
        .chain(function.params.iter().flat_map(|param| &param.annotations))
        .collect::<Vec<_>>();
    assert_eq!(
        sites
            .iter()
            .map(|site| site.name.as_str())
            .collect::<Vec<_>>(),
        [
            "Routes",
            "GET",
            "Queries",
            "RawQuery",
            "HeaderMap",
            "RawBody",
            "State",
            "Peer",
            "Extract"
        ]
    );
    for reverse in [false, true] {
        let catalog = owners(reverse);
        let mut diagnostics = Vec::new();
        for annotation in &sites {
            let ir = resolve_annotation_ir_in_namespace(
                FileId::new(30),
                annotation,
                &catalog,
                graph.scope(APP),
                &mut diagnostics,
            );
            assert_eq!(
                ir.resolved_symbol,
                Some(SymbolId::new(format!("dust_server::{}", annotation.name)))
            );
            assert_eq!(ir.raw_name, annotation.qualified_name);
            assert_eq!(ir.prefix.as_deref(), Some("s"));
            assert_eq!(ir.span, SpanIr::new(FileId::new(30), annotation.span));
        }
        assert_eq!(diagnostics, []);
    }
}
