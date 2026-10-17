use dust_ir::{
    ClassIr, ClassKindIr, NameIr, ParamKind, RedirectTargetIr, SerdeClassConfigIr,
    SerdeVariantConfigIr, SuperclassIr, TypeIr, TypeParamIr,
};
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan};
use dust_plugin_serde::register_plugin;

use super::support::{
    class, constructor, constructor_param, field, library, members_for_class, span,
};

#[path = "sealed_generics/failures.rs"]
mod failures;
#[path = "sealed_generics/manual.rs"]
mod manual;

fn parameter(name: &str, bound: Option<TypeIr>) -> TypeParamIr {
    TypeParamIr {
        name: NameIr {
            source: name.into(),
            short: name.into(),
            prefix: None,
            span: span(1, 2),
        },
        bound,
        span: span(1, 2),
    }
}

fn base() -> ClassIr {
    let mut base = class(
        "Event",
        vec![],
        vec![constructor(None, vec![])],
        &["dust_dart::Serialize", "dust_dart::Deserialize"],
    );
    base.kind = ClassKindIr::SealedClass;
    base.type_parameters = vec![
        parameter("A", None),
        parameter("B", Some(TypeIr::list_of(TypeIr::named("A")))),
    ];
    base.serde = Some(SerdeClassConfigIr {
        tag: Some("type".into()),
        variants: vec![SerdeVariantConfigIr {
            constructor_name: "done".into(),
            target_class_name: "Done".into(),
            target: Some(RedirectTargetIr {
                ty: TypeIr::generic("Done", vec![TypeIr::named("B"), TypeIr::named("A")]),
                constructor: Some(NameIr {
                    source: "create".into(),
                    short: "create".into(),
                    prefix: None,
                    span: span(5, 11),
                }),
                span: span(3, 11),
            }),
            field_serde: Default::default(),
            tag: "done".into(),
            params: vec![constructor_param(
                "label",
                TypeIr::string(),
                ParamKind::Positional,
            )],
        }],
        ..Default::default()
    });
    base
}

fn done() -> ClassIr {
    let mut done = class(
        "Done",
        vec![field("label", TypeIr::string())],
        vec![constructor(
            Some("create"),
            vec![constructor_param(
                "label",
                TypeIr::string(),
                ParamKind::Positional,
            )],
        )],
        &["dust_dart::Serialize", "dust_dart::Deserialize"],
    );
    done.type_parameters = vec![
        parameter("Y", Some(TypeIr::list_of(TypeIr::named("X")))),
        parameter("X", None),
    ];
    done.superclass_name = Some("Event".into());
    done.superclass = Some(SuperclassIr {
        ty: TypeIr::generic("Event", vec![TypeIr::named("X"), TypeIr::named("Y")]),
        span: span(2, 4),
    });
    done
}

#[test]
fn sealed_dispatch_preserves_applied_arguments_and_dependent_bounds() {
    let library = library(vec![base(), done()], vec![]);
    let plugin = register_plugin();
    assert_eq!(plugin.validate(&library), []);
    let result = plugin
        .generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        result.top_level_functions[0],
        r#"Map<String, Object?> _$EventSerialize<A, B extends List<A>>(Event<A, B> instance) {
  return switch (instance) {
    Done<B, A> value => <String, Object?>{
      ..._$DoneSerialize<B, A>(value),
      'type': 'done',
    },
    _ => throw ArgumentError.value(instance, 'instance', 'Unknown SerDe variant for Event'),
  };
}

Map<String, Object?> _$EventToJson<A, B extends List<A>>(Event<A, B> instance) =>
    _$EventSerialize<A, B>(instance);"#
    );
    assert_eq!(
        result.top_level_functions[1],
        r#"// factory Event.fromJson(Map<String, Object?> json) => _$EventFromJson(json);
Event<A, B> _$EventDeserialize<A, B extends List<A>>(Map<String, Object?> json) {
  final tagValue = JsonHelper.as<String>(json['type'], 'type', 'String');
  final variantJson = Map<String, Object?>.from(json)..remove('type');

  return switch (tagValue) {
    'done' => _$DoneDeserialize<B, A>(variantJson),
    _ => throw ArgumentError('Unknown SerDe variant tag: $tagValue'),
  };
}

Event<A, B> _$EventFromJson<A, B extends List<A>>(Map<String, Object?> json) =>
    _$EventDeserialize<A, B>(json);"#
    );
    assert_eq!(
        members_for_class(&result, "Done"),
        [
            "Map<String, Object?> serialize() =>\n    _$EventSerialize<X, Y>(this as Event<X, Y>);",
            "Map<String, Object?> toJson() => serialize();"
        ]
    );
}

#[test]
fn omitted_variants_keep_identifier_headers_named_constructors_and_applied_extends() {
    let library = library(vec![base()], vec![]);
    let plugin = register_plugin();
    assert_eq!(plugin.validate(&library), []);
    let result = plugin
        .generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        result.support_types[2],
        r#"/// A generated variant of `Event<A, B>`.
final class Done<B extends List<A>, A> extends Event<A, B> {
  /// Creates this sealed variant.
  const Done.create(
    this.label,
  ) : super();

  /// Decodes this variant's payload.
  factory Done.fromJson(Map<String, Object?> json) =>
      _$DoneFromJson<B, A>(json);

  /// The `label` payload value.
  final String label;
}"#
    );
}

#[test]
fn explicit_redirect_ctor_gets_a_collision_safe_decoder_without_changing_leaf_selection() {
    let mut done = done();
    done.constructors.insert(
        0,
        constructor(Some("normalized"), done.constructors[0].params.clone()),
    );
    let library = library(
        vec![
            base(),
            done,
            class("_dustEventDoneDeserialize", vec![], vec![], &[]),
        ],
        vec![],
    );
    let plugin = register_plugin();
    assert_eq!(plugin.validate(&library), []);
    let result = plugin
        .generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(
        result.top_level_functions.last().unwrap(),
        r#"Done<Y, X> _dustEventDoneDeserialize2<Y extends List<X>, X>(Map<String, Object?> json) {
  final labelValue = JsonHelper.as<String>(json['label'], 'label', 'String');

  return Done<Y, X>.create(labelValue);
}"#
    );
    assert_eq!(
        result.top_level_functions[3],
        r#"// factory Done.fromJson(Map<String, Object?> json) => _$DoneFromJson(json);
Done<Y, X> _$DoneDeserialize<Y extends List<X>, X>(Map<String, Object?> json) {
  final labelValue = JsonHelper.as<String>(json['label'], 'label', 'String');

  return Done<Y, X>.normalized(labelValue);
}

Done<Y, X> _$DoneFromJson<Y extends List<X>, X>(Map<String, Object?> json) =>
    _$DoneDeserialize<Y, X>(json);"#
    );
}
