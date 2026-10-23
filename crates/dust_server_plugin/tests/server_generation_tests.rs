//! Server Phase-one generation and rejection contracts.

use dust_ir::{
    AnnotationValueIr, ClassIr, ClassKindIr, ConfigApplicationIr, DartFileIr, MethodIr,
    MethodParamIr, ParamKind, PartIr, SpanIr, SymbolId, TypeIr,
};
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan};
use dust_server_plugin::register_plugin;
use dust_text::{FileId, TextRange};

fn span() -> SpanIr {
    SpanIr::new(FileId::new(1), TextRange::new(0_u32, 10_u32))
}

fn configured(symbol: &str, positional_args: Vec<AnnotationValueIr>) -> ConfigApplicationIr {
    ConfigApplicationIr::with_arguments(
        SymbolId::new(symbol),
        None,
        positional_args,
        Default::default(),
        span(),
    )
}

fn param(name: &str, ty: TypeIr, config: ConfigApplicationIr) -> MethodParamIr {
    MethodParamIr {
        name: name.to_owned(),
        ty,
        span: span(),
        kind: ParamKind::Positional,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs: vec![config],
    }
}

fn method(params: Vec<MethodParamIr>, route: &str) -> MethodIr {
    MethodIr {
        name: "get".to_owned(),
        is_static: false,
        is_external: false,
        return_type: TypeIr::generic("Future", vec![TypeIr::named("Todo")]),
        has_body: true,
        body_source: Some("=> todo;".to_owned()),
        params,
        span: span(),
        traits: Vec::new(),
        configs: vec![configured(
            "dust_server::GET",
            vec![AnnotationValueIr::String(route.to_owned())],
        )],
    }
}

fn library(method: MethodIr) -> DartFileIr {
    let mut library = DartFileIr::empty(
        ".",
        "dust_test",
        "lib/server.dart",
        "lib/server.g.dart",
        span(),
    );
    library.part_directives.push(PartIr {
        uri_literal: None,
        uri: "server.g.dart".to_owned(),
        span: span(),
    });
    library.classes.push(ClassIr {
        kind: ClassKindIr::Class,
        name: "Todos".to_owned(),
        type_parameters: Vec::new(),
        mixins: Vec::new(),
        is_abstract: false,
        is_interface: false,
        superclass_name: None,
        superclass: None,
        span: span(),
        fields: Vec::new(),
        constructors: Vec::new(),
        methods: vec![method],
        traits: Vec::new(),
        configs: vec![configured(
            "dust_server::Controller",
            vec![AnnotationValueIr::String("/todos".to_owned())],
        )],
        serde: None,
    });
    library
}

#[test]
fn generates_controller_routes_and_phase_one_extractors() {
    let mut id = param(
        "todoId",
        TypeIr::string(),
        configured("dust_server::Path", Vec::new()),
    );
    let mut done = param(
        "done",
        TypeIr::bool().nullable(),
        configured(
            "dust_server::Query",
            vec![AnnotationValueIr::String("done".to_owned())],
        ),
    );
    done.kind = ParamKind::Named;
    done.is_required = false;
    id.is_required = true;
    let library = library(method(vec![id, done], "/{todoId}"));
    let plugin = register_plugin();
    assert_eq!(plugin.validate(&library), []);
    let plan = SymbolPlan::default();
    let output = plugin.generate(&library, &PluginContext { symbol_plan: &plan });
    let members = &output[0].mixin_members[0].members;
    assert_eq!(members.len(), 2);
    assert_eq!(
        members[0],
        "Router routes() => Router.module(\n      prefix: '/todos',\n      routes: [\n        Route('GET', '/{todoId}', _$handleGet),\n      ],\n    );"
    );
    assert_eq!(
        members[1],
        r#"Future<Response> _$handleGet(Request request) async {
  final Result<String, Rejection> _dustResult0 =
      await const PathExtractable<String>('todoId').extract(request);
  if (_dustResult0 case Err(:final error)) return error.intoResponse();
  final _dustValue0 = (_dustResult0 as Ok<String, Rejection>).value;

  final Result<bool?, Rejection> _dustResult1 =
      await const QueryExtractable<bool?>('done').extract(request);
  if (_dustResult1 case Err(:final error)) return error.intoResponse();
  final _dustValue1 = (_dustResult1 as Ok<bool?, Rejection>).value;

  return guard(
    () async => responseFrom(
      await (this as Todos).get(
        _dustValue0,
        done: _dustValue1,
      ),
      status: 200,
    ),
  );
}"#
    );
}

#[test]
fn rejects_missing_path_key_and_later_phase_extractor() {
    let path = param(
        "id",
        TypeIr::string(),
        configured(
            "dust_server::Path",
            vec![AnnotationValueIr::String("id".to_owned())],
        ),
    );
    let custom = param(
        "user",
        TypeIr::named("User"),
        configured("dust_server::Extract", Vec::new()),
    );
    let messages = register_plugin()
        .validate(&library(method(vec![path, custom], "/")))
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("missing from the route"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("not supported in server Phase 1"))
    );
}
