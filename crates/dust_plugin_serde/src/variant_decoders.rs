use dust_ir::{ClassIr, ConstructorIr, DartFileIr, SerdeVariantConfigIr};

use crate::{
    emit_class::emit_constructor_decode_helper, enum_codecs::EnumCodecs, names::JsonNamePlan,
    writer::find_deserialize_constructor,
};

/// Selects the constructor actually named by parser-owned redirect facts.
pub(crate) fn redirect_constructor<'a>(
    library: &'a DartFileIr,
    variant: &SerdeVariantConfigIr,
) -> Option<(&'a ClassIr, &'a ConstructorIr)> {
    let target = variant.target.as_ref()?;
    let class = library
        .classes
        .iter()
        .find(|class| class.name == variant.target_class_name)?;
    let name = target
        .constructor
        .as_ref()
        .filter(|name| name.source != "new")
        .map(|name| name.source.as_str());
    let constructor = class
        .constructors
        .iter()
        .find(|constructor| constructor.name.as_deref() == name)?;
    Some((class, constructor))
}

/// Ordinary leaf decoding keeps its existing first-constructible-ctor contract.
pub(crate) fn needs_selected_decoder(
    base: &ClassIr,
    variant: &SerdeVariantConfigIr,
    library: &DartFileIr,
) -> bool {
    if !wants_deserialize(base) {
        return false;
    }
    let Some((class, constructor)) = redirect_constructor(library, variant) else {
        return false;
    };
    wants_deserialize(class)
        && find_deserialize_constructor(class).is_some_and(|ordinary| ordinary != constructor)
}

/// Emits only the extra payload helpers that a differing redirect constructor needs.
pub(crate) fn emit_selected_decoders(
    library: &DartFileIr,
    names: &JsonNamePlan,
    classes: &[&str],
    enums: EnumCodecs<'_>,
) -> Vec<String> {
    let mut result = Vec::new();
    for base in &library.classes {
        let Some(serde) = &base.serde else {
            continue;
        };
        for variant in &serde.variants {
            let Some(name) = names.variant_decoder(base, variant) else {
                continue;
            };
            if let Some((class, constructor)) = redirect_constructor(library, variant)
                && let Some(helper) = emit_constructor_decode_helper(
                    class,
                    constructor,
                    name,
                    classes,
                    enums,
                    names.names_for(class),
                )
            {
                result.push(helper);
            }
        }
    }
    result
}

/// A handwritten leaf decoder remains responsible for its own construction semantics.
pub(crate) fn wants_deserialize(class: &ClassIr) -> bool {
    class
        .traits
        .iter()
        .any(|trait_| trait_.symbol.0 == "dust_dart::Deserialize")
}
