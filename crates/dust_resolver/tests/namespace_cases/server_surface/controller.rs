use super::*;
use dust_resolver::resolve_library_in_namespace;

#[test]
fn server_class_method_and_parameter_configs_keep_exact_owners_and_spans() {
    let source = format!(
        r#"import '{BARREL}' as s;
import '{CLIENT}' as c;
part 'package:app/main.g.dart';
@s.Controller('/todos')
class Api {{
  @s.GET('/{{id}}')
  void read(@s.Path() String id, @s.Query('q') String q,
      @s.Header('x') String header, @s.Body() Object body) {{}}
  @s.POST('/') @s.FormUrlEncoded()
  void write(@s.Field('title') String title) {{}}
  @s.POST('/upload') @s.MultiPart()
  void upload(@s.Part('file') Object file) {{}}
}}
class Client {{
  @c.GET('/{{id}}')
  void read(@c.Path() String id, @c.Query('q') String q,
      @c.Header('x') String header, @c.Body() Object body) {{}}
}}
"#
    );
    let parsed = parse(30, &source);
    assert_eq!(
        parsed
            .classes
            .iter()
            .map(|class| class.name.as_str())
            .collect::<Vec<_>>(),
        ["Api", "Client"]
    );
    assert_eq!(
        parsed.classes[0].annotations[0].qualified_name,
        "s.Controller"
    );
    assert_eq!(
        parsed
            .classes
            .iter()
            .flat_map(|class| &class.methods)
            .map(|method| (
                method.name.as_str(),
                method.annotations.len(),
                method.params.len()
            ))
            .collect::<Vec<_>>(),
        [
            ("read", 1, 4),
            ("write", 2, 1),
            ("upload", 2, 1),
            ("read", 1, 4)
        ]
    );
    let mut graph = surface();
    add(&mut graph, 30, APP, &source);
    add(
        &mut graph,
        31,
        "package:app/main.g.dart",
        &format!("part of '{APP}';"),
    );
    for reverse in [false, true] {
        let catalog = owners(reverse);
        let result = resolve_library_in_namespace(
            FileId::new(30),
            "main.dart",
            "main.g.dart",
            &parsed,
            &catalog,
            graph.scope(APP),
            &[],
        );
        assert_eq!(result.diagnostics, []);
        assert_eq!(result.library.classes.len(), parsed.classes.len());
        for (class, original) in result.library.classes.iter().zip(&parsed.classes) {
            assert_configs(&class.configs, &original.annotations);
            assert_eq!(class.methods.len(), original.methods.len());
            for (method, original) in class.methods.iter().zip(&original.methods) {
                assert_configs(&method.configs, &original.annotations);
                assert_eq!(method.params.len(), original.params.len());
                for (param, original) in method.params.iter().zip(&original.params) {
                    assert_configs(&param.configs, &original.annotations);
                }
            }
        }
    }
}

/// Exact retained ownership, written arguments and original metadata coordinates.
fn assert_configs(
    configs: &[dust_ir::ConfigApplicationIr],
    annotations: &[dust_parser_dart::ParsedAnnotation],
) {
    assert_eq!(configs.len(), annotations.len());
    for (config, annotation) in configs.iter().zip(annotations) {
        let (prefix, name) = annotation.qualified_name.split_once('.').unwrap();
        let owner = if prefix == "s" {
            "dust_server"
        } else {
            "dust_dart"
        };
        assert_eq!(config.symbol, SymbolId::new(format!("{owner}::{name}")));
        assert_eq!(config.span, SpanIr::new(FileId::new(30), annotation.span));
        assert_eq!(config.arguments_source, annotation.arguments_source);
    }
}
