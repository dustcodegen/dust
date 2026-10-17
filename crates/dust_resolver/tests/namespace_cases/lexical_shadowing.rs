use super::*;
use dust_resolver::resolve_library_in_namespace;

#[test]
fn class_constant_metadata_cannot_become_an_imported_client_verb() {
    let mut graph = graph();
    let source = format!(
        "import '{CLIENT}'; class Api {{ static const GET = Object(); @GET void call() {{}} }}"
    );
    add(&mut graph, 3, APP, &source);
    let result = resolve_library_in_namespace(
        FileId::new(3),
        "main.dart",
        "main.g.dart",
        &parse(3, &source),
        &catalog(false),
        graph.scope(APP),
        &["dust_dart::GET"],
    );
    assert_eq!(result.diagnostics, []);
    assert_eq!(result.library.classes[0].methods[0].configs, []);
}
