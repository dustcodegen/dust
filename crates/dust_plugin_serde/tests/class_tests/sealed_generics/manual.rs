use super::*;

#[test]
fn manual_variants_keep_typed_json_method_fallbacks() {
    let mut done = done();
    done.traits.clear();
    let library = library(vec![base(), done], vec![]);
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
      ...value.toJson(),
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
    'done' => Done<B, A>.fromJson(variantJson),
    _ => throw ArgumentError('Unknown SerDe variant tag: $tagValue'),
  };
}

Event<A, B> _$EventFromJson<A, B extends List<A>>(Map<String, Object?> json) =>
    _$EventDeserialize<A, B>(json);"#
    );
}

#[test]
fn serialize_only_bases_do_not_plan_constructor_specific_decoders() {
    let mut base = base();
    base.traits
        .retain(|trait_| trait_.symbol.0 != "dust_dart::Deserialize");
    let mut done = done();
    done.constructors.insert(
        0,
        constructor(Some("ordinary"), done.constructors[0].params.clone()),
    );
    let library = library(vec![base, done], vec![]);
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
    assert_eq!(result.top_level_functions.len(), 3);
}

#[test]
fn defensive_emission_omits_payload_helpers_when_decode_arguments_are_unavailable() {
    let mut done = done();
    done.constructors[0].params.push(constructor_param(
        "extra",
        TypeIr::int(),
        ParamKind::Positional,
    ));
    let library = library(vec![done.clone()], vec![]);
    let plugin = register_plugin();
    let result = plugin
        .generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(result.top_level_functions.len(), 1);

    let mut ordinary = constructor(Some("ordinary"), done.constructors[0].params.clone());
    ordinary.params.pop();
    done.constructors.insert(0, ordinary);
    let library = super::library(vec![base(), done], vec![]);
    assert_eq!(
        plugin.validate(&library)[0].message,
        "Dust cannot generate sealed variant `Event.done`: redirect constructor cannot initialize every decoded variant field"
    );
    let result = plugin
        .generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .remove(0);
    assert_eq!(result.top_level_functions.len(), 4);
}
