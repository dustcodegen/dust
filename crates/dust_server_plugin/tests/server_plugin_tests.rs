//! Server plugin Phase-0 contract tests.

use dust_ir::{
    AnnotationIr, ClassIr, ClassKindIr, ConfigApplicationIr, DartFileIr, FunctionIr, MethodIr,
    MethodParamIr, NameIr, ParamKind, SpanIr, SymbolId, TypeIr,
};
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan};
use dust_server_plugin::{ServerPlugin, register_plugin};
use dust_text::{FileId, TextRange};

fn empty_library() -> DartFileIr {
    DartFileIr::empty(
        ".",
        "dust_test",
        "lib/server.dart",
        "lib/server.g.dart",
        SpanIr::new(FileId::new(1), TextRange::new(0_u32, 10_u32)),
    )
}

fn config(symbol: &str, span: SpanIr) -> ConfigApplicationIr {
    ConfigApplicationIr::new(SymbolId::new(symbol), None, span)
}

fn annotation(symbol: &str, span: SpanIr) -> AnnotationIr {
    let short_name = symbol.rsplit_once("::").unwrap().1.to_owned();
    AnnotationIr {
        raw_name: short_name.clone(),
        short_name,
        prefix: None,
        positional_args: Vec::new(),
        named_args: Default::default(),
        resolved_symbol: Some(SymbolId::new(symbol)),
        span,
    }
}

fn param(configs: Vec<ConfigApplicationIr>, span: SpanIr) -> MethodParamIr {
    MethodParamIr {
        name: "id".to_owned(),
        ty: TypeIr::string(),
        span,
        kind: ParamKind::Positional,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs,
    }
}

fn method(configs: Vec<ConfigApplicationIr>, params: Vec<MethodParamIr>, span: SpanIr) -> MethodIr {
    MethodIr {
        name: "read".to_owned(),
        is_static: false,
        is_external: false,
        return_type: TypeIr::string(),
        has_body: true,
        body_source: Some("=> 'ok';".to_owned()),
        params,
        span,
        traits: Vec::new(),
        configs,
    }
}

fn class(configs: Vec<ConfigApplicationIr>, methods: Vec<MethodIr>, span: SpanIr) -> ClassIr {
    ClassIr {
        kind: ClassKindIr::Class,
        name: "Todos".to_owned(),
        type_parameters: Vec::new(),
        mixins: Vec::new(),
        is_abstract: false,
        is_interface: false,
        superclass_name: None,
        superclass: None,
        span,
        fields: Vec::new(),
        constructors: Vec::new(),
        methods,
        traits: Vec::new(),
        configs,
        serde: None,
    }
}

fn function(
    annotations: Vec<AnnotationIr>,
    params: Vec<MethodParamIr>,
    span: SpanIr,
) -> FunctionIr {
    FunctionIr {
        name: NameIr {
            source: "read".to_owned(),
            short: "read".to_owned(),
            prefix: None,
            span,
        },
        return_type: TypeIr::string(),
        params,
        annotations,
        span,
    }
}

fn assert_rejected(file: &DartFileIr) {
    let diagnostics = register_plugin().validate(file);
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.labels.is_empty())
    );
}

#[test]
fn registration_exposes_the_exact_phase_zero_contract() {
    let plugin = register_plugin();
    assert_eq!(plugin.plugin_name(), "Server");
    assert!(plugin.claimed_traits().is_empty());
    assert_eq!(plugin.claimed_configs().len(), 24);
    assert_eq!(plugin.supported_annotations().len(), 24);
    assert_eq!(plugin.annotation_origins().len(), 24);
    assert_eq!(plugin.partless_configs(), plugin.claimed_configs());
    assert_eq!(
        plugin
            .annotation_origins()
            .iter()
            .map(|origin| origin.symbol)
            .collect::<Vec<_>>(),
        plugin.claimed_configs()
    );
}

#[test]
fn unannotated_libraries_validate_and_generate_nothing() {
    let plugin = ServerPlugin::new();
    let file = empty_library();
    assert_eq!(plugin.validate(&file), []);

    let plan = SymbolPlan::default();
    let output = plugin.generate(&file, &PluginContext { symbol_plan: &plan });
    assert_eq!(output.len(), 1);
    assert!(output[0].is_empty());
    assert_eq!(output[0].diagnostics, []);
}

#[test]
fn invalid_libraries_generate_diagnostics_instead_of_code() {
    let plugin = register_plugin();
    let mut file = empty_library();
    file.library_annotations = vec![annotation("dust_server::Routes", file.span)];
    let plan = SymbolPlan::default();

    let output = plugin.generate(&file, &PluginContext { symbol_plan: &plan });

    assert_eq!(output.len(), 1);
    assert!(!output[0].diagnostics.is_empty());
    assert!(output[0].mixin_members.is_empty());
    assert!(output[0].top_level_functions.is_empty());
}

#[test]
fn incomplete_server_declarations_fail_clearly() {
    let mut library = empty_library();
    let span = library.span;

    library.library_annotations = vec![annotation("dust_server::Routes", span)];
    assert_rejected(&library);

    library.library_annotations.clear();
    library.classes = vec![class(
        vec![config("dust_server::Controller", span)],
        Vec::new(),
        span,
    )];
    assert_rejected(&library);

    library.classes = vec![class(
        Vec::new(),
        vec![method(
            vec![config("dust_server::GET", span)],
            Vec::new(),
            span,
        )],
        span,
    )];
    assert_rejected(&library);

    library.classes = vec![class(
        Vec::new(),
        vec![method(
            Vec::new(),
            vec![param(vec![config("dust_server::Path", span)], span)],
            span,
        )],
        span,
    )];
    assert_rejected(&library);

    library.classes.clear();
    library.functions = vec![function(
        vec![annotation("dust_server::GET", span)],
        Vec::new(),
        span,
    )];
    assert_rejected(&library);

    library.functions = vec![function(
        Vec::new(),
        vec![param(vec![config("dust_server::Query", span)], span)],
        span,
    )];
    assert_rejected(&library);
}

#[test]
fn overlapping_client_symbols_do_not_trigger_server_validation() {
    let mut library = empty_library();
    let span = library.span;
    library.classes = vec![class(
        Vec::new(),
        vec![method(
            vec![config("dust_dart::GET", span)],
            Vec::new(),
            span,
        )],
        span,
    )];

    assert_eq!(register_plugin().validate(&library), []);
}
