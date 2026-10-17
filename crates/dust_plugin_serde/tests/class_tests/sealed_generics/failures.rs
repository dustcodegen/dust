use super::*;
use dust_diagnostics::{Diagnostic, SourceLabel};

fn assert_error(library: &dust_ir::DartFileIr, reason: &str) {
    assert_eq!(
        register_plugin().validate(library),
        [Diagnostic::error(format!(
            "Dust cannot generate sealed variant `Event.done`: {reason}"
        ))
        .with_label(SourceLabel::new(
            span(3, 11).file_id,
            span(3, 11).range,
            "unsupported sealed target application"
        ))]
    );
}

#[test]
fn declared_variant_applications_fail_closed_on_unproven_or_opaque_substitutions() {
    for (case, reason) in [
        (0, "wrong redirect target argument count"),
        (1, "redirect target ownership is not proven in this library"),
        (2, "redirect target ownership is not proven in this library"),
        (3, "redirect requires a non-nullable named class"),
        (
            4,
            "target superclass arguments do not preserve the owning base application",
        ),
        (5, "missing applied superclass facts"),
        (6, "wrong superclass argument count"),
        (7, "opaque function type"),
        (8, "opaque record type"),
        (9, "unresolved type"),
        (10, "non-class superclass"),
        (11, "unproven local superclass"),
        (12, "cyclic superclass application"),
        (
            13,
            "redirect constructor is not declared on the target class",
        ),
        (
            14,
            "redirect constructor cannot initialize every decoded variant field",
        ),
        (
            15,
            "redirect constructor cannot initialize every decoded variant field",
        ),
    ] {
        let mut library = library(vec![base(), done()], vec![]);
        let variant = &mut library.classes[0].serde.as_mut().unwrap().variants[0];
        match case {
            0 => variant.target.as_mut().unwrap().ty = TypeIr::named("Done"),
            1 => variant.target.as_mut().unwrap().ty = TypeIr::named("Other"),
            2 => {
                variant.target_class_name = "p.Done".into();
                variant.target.as_mut().unwrap().ty = TypeIr::named("p.Done");
            }
            3 => variant.target.as_mut().unwrap().ty = TypeIr::dynamic(),
            4 => {
                variant.target.as_mut().unwrap().ty =
                    TypeIr::generic("Done", vec![TypeIr::named("A"), TypeIr::named("B")])
            }
            5 => library.classes[1].superclass = None,
            6 => {
                library.classes[1].superclass.as_mut().unwrap().ty =
                    TypeIr::generic("Event", vec![TypeIr::named("X")])
            }
            7 => {
                library.classes[1].superclass.as_mut().unwrap().ty =
                    TypeIr::function("X Function(Y)")
            }
            8 => library.classes[1].superclass.as_mut().unwrap().ty = TypeIr::record("(X, Y)"),
            9 => library.classes[1].superclass.as_mut().unwrap().ty = TypeIr::unknown(),
            10 => library.classes[1].superclass.as_mut().unwrap().ty = TypeIr::dynamic(),
            11 => library.classes[1].superclass.as_mut().unwrap().ty = TypeIr::named("Missing"),
            12 => {
                library.classes[1].superclass.as_mut().unwrap().ty =
                    TypeIr::generic("Done", vec![TypeIr::named("Y"), TypeIr::named("X")])
            }
            13 => library.classes[1].constructors[0].name = Some("other".into()),
            14 => {
                let params = library.classes[1].constructors[0].params.clone();
                library.classes[1].constructors[0].params.clear();
                library.classes[1]
                    .constructors
                    .push(constructor(Some("other"), params));
            }
            _ => library.classes[1].constructors[0]
                .params
                .push(constructor_param(
                    "extra",
                    TypeIr::string(),
                    ParamKind::Positional,
                )),
        }
        assert_error(&library, reason);
    }
}

#[test]
fn missing_declarations_never_invent_binders_or_rewrite_applied_arguments() {
    for (arguments, reason) in [
        (
            vec![],
            "generated target must carry every owning type parameter explicitly",
        ),
        (
            vec![TypeIr::int(), TypeIr::named("A")],
            "generated target arguments must be bare owning type parameters",
        ),
        (
            vec![TypeIr::named("B").nullable(), TypeIr::named("A")],
            "generated target arguments must be bare owning type parameters",
        ),
        (
            vec![
                TypeIr::generic("B", vec![TypeIr::named("A")]),
                TypeIr::named("A"),
            ],
            "generated target arguments must be bare owning type parameters",
        ),
        (
            vec![TypeIr::named("B"), TypeIr::named("B")],
            "generated target arguments must be distinct owning type parameters",
        ),
        (
            vec![TypeIr::named("Z"), TypeIr::named("A")],
            "generated target argument has no owning source binder",
        ),
    ] {
        let mut library = library(vec![base()], vec![]);
        library.classes[0].serde.as_mut().unwrap().variants[0]
            .target
            .as_mut()
            .unwrap()
            .ty = TypeIr::generic("Done", arguments);
        assert_error(&library, reason);
    }
}

#[test]
fn applied_ancestor_chains_keep_each_intermediate_binder_scope() {
    let mut done = done();
    done.superclass.as_mut().unwrap().ty =
        TypeIr::generic("Mid", vec![TypeIr::named("X"), TypeIr::named("Y")]);
    let mut mid = class("Mid", vec![], vec![constructor(None, vec![])], &[]);
    mid.type_parameters = vec![
        parameter("P", None),
        parameter("Q", Some(TypeIr::list_of(TypeIr::named("P")))),
    ];
    mid.superclass = Some(SuperclassIr {
        ty: TypeIr::generic("Event", vec![TypeIr::named("P"), TypeIr::named("Q")]),
        span: span(2, 4),
    });
    let library = library(vec![base(), done, mid], vec![]);
    assert_eq!(register_plugin().validate(&library), []);
    let result = register_plugin()
        .generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        members_for_class(&result, "Done"),
        [
            "Map<String, Object?> serialize() =>\n    _$EventSerialize<X, Y>(this as Event<X, Y>);",
            "Map<String, Object?> toJson() => serialize();"
        ]
    );
}
