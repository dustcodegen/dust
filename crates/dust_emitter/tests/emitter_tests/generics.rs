use std::fs;

use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_emitter::{emit_library, write_library};
use dust_ir::{MixinApplicationIr, NameIr, TypeIr, TypeParamIr};
use dust_plugin_api::{PluginContribution, PluginRegistry};
use tempfile::tempdir;

use super::support::{FakePlugin, generated_output, sample_library, span};

fn parameter(name: &str, bound: Option<TypeIr>) -> TypeParamIr {
    TypeParamIr {
        name: NameIr {
            source: name.to_owned(),
            short: name.to_owned(),
            prefix: None,
            span: span(1, 2),
        },
        bound,
        span: span(1, 8),
    }
}

fn registry() -> PluginRegistry {
    let mut contribution = PluginContribution::default();
    contribution.push_mixin_interface("User", "Runnable");
    contribution.push_mixin_member("User", "T get value;");
    let mut registry = PluginRegistry::new();
    registry
        .register(Box::new(FakePlugin {
            name: "generic",
            requested: Vec::new(),
            diagnostics: Vec::new(),
            contribution,
        }))
        .unwrap();
    registry
}

fn application(ty: TypeIr) -> MixinApplicationIr {
    MixinApplicationIr {
        ty,
        span: span(40, 60),
    }
}

#[test]
fn generic_mixin_declarations_preserve_order_and_bounds() {
    let mut library = sample_library(String::new());
    library.classes[0].type_parameters = vec![
        parameter("T", Some(TypeIr::num())),
        parameter("U", Some(TypeIr::list_of(TypeIr::named("T").nullable()))),
    ];
    library.classes[0].mixins = vec![application(TypeIr::generic(
        "_$User",
        vec![TypeIr::named("T"), TypeIr::named("U")],
    ))];
    let emitted = emit_library(&library, &registry(), None);
    assert_eq!(emitted.diagnostics, []);
    assert_eq!(
        emitted.source,
        generated_output(
            r#"part of 'user.dart';

mixin _$User<T extends num, U extends List<T?>> implements Runnable {
  T get value;
}
"#
        )
    );
}

#[test]
fn generic_mixin_arguments_must_forward_each_owning_binder_once_in_order() {
    for applied in [
        vec![],
        vec![TypeIr::named("_$User")],
        vec![TypeIr::generic(
            "_$User",
            vec![TypeIr::int(), TypeIr::named("U")],
        )],
        vec![TypeIr::generic(
            "_$User",
            vec![TypeIr::named("U"), TypeIr::named("T")],
        )],
        vec![TypeIr::generic(
            "_$User",
            vec![TypeIr::named("T").nullable(), TypeIr::named("U")],
        )],
        vec![TypeIr::generic("_$User", vec![TypeIr::named("T")])],
        vec![TypeIr::generic(
            "p._$User",
            vec![TypeIr::named("T"), TypeIr::named("U")],
        )],
        vec![TypeIr::generic("_$User", vec![TypeIr::named("T"), TypeIr::named("U")]); 2],
    ] {
        let mut library = sample_library(String::new());
        library.classes[0].type_parameters = vec![parameter("T", None), parameter("U", None)];
        library.classes[0].mixins = applied.into_iter().map(application).collect();
        let location = library.classes[0]
            .mixins
            .iter()
            .find(|mixin| mixin.ty.name() == Some("_$User"))
            .map(|mixin| mixin.span)
            .unwrap_or(library.classes[0].span);
        let expected = Diagnostic::error("Generic model `User` must apply `with _$User<T, U>`")
            .with_label(SourceLabel::new(
                location.file_id,
                location.range,
                "forward the class type parameters in declaration order",
            ));
        let emitted = emit_library(&library, &registry(), None);
        assert_eq!(emitted.diagnostics, [expected]);
    }
}

#[test]
fn generic_classes_without_generated_mixin_contributions_are_not_rejected() {
    let mut library = sample_library(String::new());
    library.classes[0].type_parameters = vec![parameter("T", None)];
    let emitted = emit_library(&library, &PluginRegistry::new(), None);
    assert_eq!(emitted.diagnostics, []);
    assert_eq!(emitted.source, "");
}

#[test]
fn interface_only_contributions_require_forwarded_generic_arguments() {
    let mut library = sample_library(String::new());
    library.classes[0].type_parameters = vec![parameter("T", None)];
    let mut contribution = PluginContribution::default();
    contribution.push_mixin_interface("User", "Runnable");
    let mut registry = PluginRegistry::new();
    registry
        .register(Box::new(FakePlugin {
            name: "interface-only",
            requested: Vec::new(),
            diagnostics: Vec::new(),
            contribution,
        }))
        .unwrap();
    let emitted = emit_library(&library, &registry, None);
    assert_eq!(emitted.diagnostics.len(), 1);
    assert_eq!(
        emitted.diagnostics[0].message,
        "Generic model `User` must apply `with _$User<T>`"
    );
}

#[test]
fn invalid_mixin_arguments_do_not_create_or_overwrite_outputs() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("user.g.dart");
    let mut library = sample_library(output.display().to_string());
    library.classes[0].type_parameters = vec![parameter("T", None)];
    let rejected = write_library(&library, &registry()).unwrap();
    assert!(!rejected.written);
    assert!(!output.exists());
    fs::write(&output, "previous generated output\n").unwrap();
    let rejected = write_library(&library, &registry()).unwrap();
    assert!(!rejected.written);
    assert_eq!(
        fs::read_to_string(output).unwrap(),
        "previous generated output\n"
    );
}
