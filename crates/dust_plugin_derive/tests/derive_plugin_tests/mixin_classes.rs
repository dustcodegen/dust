use dust_ir::ClassKindIr;
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan};
use dust_plugin_derive::register_plugin;

use super::support::{class_with_traits, sample_library};

#[test]
fn plain_and_foreign_trait_mixin_classes_do_not_block_ordinary_derives() {
    let plugin = register_plugin();
    let plan = SymbolPlan::default();
    let context = PluginContext { symbol_plan: &plan };
    let original = sample_library(&["dust_dart::ToString"]);
    let expected = plugin.generate(&original, &context);
    for traits in [
        vec![],
        vec!["dust_dart::Serialize"],
        vec!["other::ToString"],
    ] {
        let mut library = original.clone();
        let mut helper = class_with_traits("Helper", &traits);
        helper.kind = ClassKindIr::MixinClass;
        library.classes.push(helper);
        assert_eq!(plugin.validate(&library), [], "{traits:?}");
        assert_eq!(plugin.validate_with_plan(&library, &plan), [], "{traits:?}");
        assert_eq!(plugin.generate(&library, &context), expected, "{traits:?}");
    }
}

#[test]
fn every_owned_derive_symbol_still_rejects_mixin_class_targets() {
    let plugin = register_plugin();
    for symbol in plugin.claimed_traits() {
        let mut library = sample_library(&[]);
        let mut target = class_with_traits("Helper", &[symbol]);
        target.kind = ClassKindIr::MixinClass;
        library.classes = vec![target];
        let diagnostics = plugin.validate(&library);
        assert_eq!(diagnostics.len(), 1, "{symbol}");
        assert_eq!(
            diagnostics[0].message,
            "Dust derive generation does not support `mixin class` targets like `Helper`",
            "{symbol}"
        );
    }
}

#[test]
fn every_owned_derive_symbol_rejects_implicit_mixin_application_constructors() {
    let plugin = register_plugin();
    for symbol in plugin.claimed_traits() {
        let mut library = sample_library(&[]);
        let mut target = class_with_traits("Helper", &[symbol]);
        target.kind = ClassKindIr::MixinApplicationClass;
        library.classes = vec![target];
        assert_eq!(
            plugin
                .validate(&library)
                .iter()
                .map(|error| error.message.as_str())
                .collect::<Vec<_>>(),
            [
                "Dust derive generation does not support `named mixin-application class` targets like `Helper`"
            ],
            "{symbol}"
        );
    }
}

#[test]
fn foreign_named_mixin_applications_do_not_change_other_derive_output() {
    let plugin = register_plugin();
    let plan = SymbolPlan::default();
    let context = PluginContext { symbol_plan: &plan };
    let original = sample_library(&["dust_dart::ToString"]);
    let expected = plugin.generate(&original, &context);
    for traits in [
        vec![],
        vec!["dust_dart::Serialize"],
        vec!["other::ToString"],
    ] {
        let mut library = original.clone();
        let mut helper = class_with_traits("Helper", &traits);
        helper.kind = ClassKindIr::MixinApplicationClass;
        library.classes.push(helper);
        assert_eq!(plugin.validate(&library), []);
        assert_eq!(plugin.generate(&library, &context), expected);
    }
}
