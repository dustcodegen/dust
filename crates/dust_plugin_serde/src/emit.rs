use std::collections::HashMap;

use dust_ir::{ClassIr, DartFileIr, EnumIr, TypeIr};
use dust_plugin_api::{PluginContribution, WorkspaceAnalysis};

use crate::{
    analysis::{JSON_DESERIALIZABLE_ENUMS_KEY, JSON_SERIALIZABLE_ENUMS_KEY},
    emit_class::{
        emit_deserializer_support_type, emit_from_json_helper, emit_serialize_mixin_members,
        emit_serializer_support_type, emit_to_json_helper,
    },
    emit_enum::{
        emit_enum_deserializer_support_type, emit_enum_from_json_helper,
        emit_enum_serializer_support_type, emit_enum_to_json_helper,
    },
    emit_sealed::{
        emit_sealed_from_json_helper, emit_sealed_to_json_helper, is_sealed_serde_class,
    },
    emit_variant_class::{emit_generated_variant_class, generated_variant_classes},
    enum_codecs::EnumCodecs,
    names::plan_json_names,
};

/// Orchestrates the emission of all SerDe-related code for a library.
///
/// This function identifies which models (classes and enums) have requested
/// serialization or deserialization and generates the corresponding Dart code.
pub(crate) fn emit_library(
    library: &DartFileIr,
    analysis: &WorkspaceAnalysis,
) -> PluginContribution {
    let mut contribution = PluginContribution::default();
    let generated_variants = generated_variant_classes(library);
    let json_names = plan_json_names(
        library,
        &generated_variants
            .iter()
            .map(|v| &v.class)
            .collect::<Vec<_>>(),
    );
    contribution
        .shared_helpers
        .extend(json_names.declarations.iter().cloned());
    let mut serializable_classes = library
        .classes
        .iter()
        .filter(|class| wants_serialize(class))
        .map(|class| class.name.as_str())
        .collect::<Vec<_>>();
    let serializable_enums = library
        .enums
        .iter()
        .filter(|e| wants_serialize_enum(e))
        .map(|e| e.name.as_str())
        .collect::<Vec<_>>();

    serializable_classes.extend(
        generated_variants
            .iter()
            .filter(|variant| variant.serializable)
            .map(|variant| variant.class.name.as_str()),
    );

    let mut deserializable_classes = library
        .classes
        .iter()
        .filter(|class| wants_deserialize(class))
        .map(|class| class.name.as_str())
        .collect::<Vec<_>>();
    let deserializable_enums = library
        .enums
        .iter()
        .filter(|e| wants_deserialize_enum(e))
        .map(|e| e.name.as_str())
        .collect::<Vec<_>>();
    deserializable_classes.extend(
        generated_variants
            .iter()
            .filter(|variant| variant.deserializable)
            .map(|variant| variant.class.name.as_str()),
    );
    let sealed_base_by_variant = sealed_base_by_variant(library);
    // Enums elsewhere in the workspace are reachable only through their public
    // support types, so a use site needs Pass 2's view as well as its own.
    let serializable_enums = EnumCodecs {
        local: &serializable_enums,
        workspace: analysis
            .string_set(JSON_SERIALIZABLE_ENUMS_KEY)
            .unwrap_or_default(),
    };
    let deserializable_enums = EnumCodecs {
        local: &deserializable_enums,
        workspace: analysis
            .string_set(JSON_DESERIALIZABLE_ENUMS_KEY)
            .unwrap_or_default(),
    };

    // Generate class-specific code.
    for class in &library.classes {
        let names = json_names.names_for(class);
        if wants_serialize(class) {
            let helper_class_name = sealed_base_by_variant
                .get(class.name.as_str())
                .and_then(|ty| ty.name())
                .unwrap_or(class.name.as_str());
            contribution.push_mixin_interface(&class.name, names.reference("Serializable"));
            for member in emit_serialize_mixin_members(
                class,
                helper_class_name,
                sealed_base_by_variant.get(class.name.as_str()),
                names,
            ) {
                contribution.push_mixin_member(&class.name, member);
            }
            if is_sealed_serde_class(class) {
                if let Some(helper) =
                    emit_sealed_to_json_helper(class, &serializable_classes, names)
                {
                    contribution
                        .support_types
                        .push(emit_serializer_support_type(class, names));
                    contribution.top_level_functions.push(helper);
                }
            } else {
                contribution
                    .support_types
                    .push(emit_serializer_support_type(class, names));
                contribution.top_level_functions.push(emit_to_json_helper(
                    class,
                    &serializable_classes,
                    serializable_enums,
                    names,
                ));
            }
        }
        if wants_deserialize(class) {
            let mut emitted_helper = false;
            if is_sealed_serde_class(class) {
                if let Some(helper) =
                    emit_sealed_from_json_helper(class, &deserializable_classes, names, &json_names)
                {
                    contribution.top_level_functions.push(helper);
                    emitted_helper = true;
                }
            } else if let Some(helper) =
                emit_from_json_helper(class, &deserializable_classes, deserializable_enums, names)
            {
                contribution.top_level_functions.push(helper);
                emitted_helper = true;
            }
            if emitted_helper {
                contribution
                    .support_types
                    .push(emit_deserializer_support_type(class, names));
            }
        }
    }

    let generated_variant_support = generated_variants
        .iter()
        .map(|variant| emit_generated_variant_class(variant, json_names.names_for(&variant.class)))
        .collect::<Vec<_>>()
        .join("\n\n");
    if !generated_variant_support.is_empty() {
        contribution.support_types.push(generated_variant_support);
    }

    for variant in &generated_variants {
        let names = json_names.names_for(&variant.class);
        if variant.serializable {
            contribution
                .support_types
                .push(emit_serializer_support_type(&variant.class, names));
            contribution.top_level_functions.push(emit_to_json_helper(
                &variant.class,
                &serializable_classes,
                serializable_enums,
                names,
            ));
        }
        if variant.deserializable {
            if let Some(helper) = emit_from_json_helper(
                &variant.class,
                &deserializable_classes,
                deserializable_enums,
                names,
            ) {
                contribution.top_level_functions.push(helper);
            }
            contribution
                .support_types
                .push(emit_deserializer_support_type(&variant.class, names));
        }
    }

    contribution
        .top_level_functions
        .extend(crate::variant_decoders::emit_selected_decoders(
            library,
            &json_names,
            &deserializable_classes,
            deserializable_enums,
        ));

    // Generate enum-specific code.
    for e in &library.enums {
        if wants_serialize_enum(e) {
            contribution
                .support_types
                .push(emit_enum_serializer_support_type(&e.name));
            contribution
                .top_level_functions
                .push(emit_enum_to_json_helper(e));
        }
        if wants_deserialize_enum(e) {
            contribution
                .support_types
                .push(emit_enum_deserializer_support_type(&e.name));
            contribution
                .top_level_functions
                .push(emit_enum_from_json_helper(e));
        }
    }
    contribution
}

/// Maps sealed variant target class names back to their sealed base class.
fn sealed_base_by_variant(library: &DartFileIr) -> HashMap<&str, TypeIr> {
    let mut base_by_variant = HashMap::new();
    for class in &library.classes {
        let Some(serde) = &class.serde else {
            continue;
        };
        for variant in &serde.variants {
            if let Some(target) = library
                .classes
                .iter()
                .find(|target| target.name == variant.target_class_name)
                && let Ok(ty) = crate::variant_types::base_type(target, class, library)
            {
                base_by_variant.insert(variant.target_class_name.as_str(), ty);
            }
        }
    }
    base_by_variant
}

/// Returns true when a class requests JSON serialization.
fn wants_serialize(class: &ClassIr) -> bool {
    class
        .traits
        .iter()
        .any(|item| item.symbol.0 == "dust_dart::Serialize")
}

/// Returns true when a class requests JSON deserialization.
fn wants_deserialize(class: &ClassIr) -> bool {
    class
        .traits
        .iter()
        .any(|item| item.symbol.0 == "dust_dart::Deserialize")
}

/// Returns true when an enum requests JSON serialization.
fn wants_serialize_enum(e: &EnumIr) -> bool {
    e.traits
        .iter()
        .any(|item| item.symbol.0 == "dust_dart::Serialize")
}

/// Returns true when an enum requests JSON deserialization.
fn wants_deserialize_enum(e: &EnumIr) -> bool {
    e.traits
        .iter()
        .any(|item| item.symbol.0 == "dust_dart::Deserialize")
}
