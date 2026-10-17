pub(crate) use dust_dart_emit::{
    NameAllocator, library_declaration_names, library_reference_names,
};

use dust_ir::ClassIr;

/// Lowercases the first ASCII character in a generated identifier fragment.
pub(crate) fn class_helper_stem(class: &ClassIr) -> String {
    // Generic helpers normalize private names without changing legacy output.
    lower_first(if class.type_parameters.is_empty() {
        &class.name
    } else {
        class.name.trim_start_matches('_')
    })
}

/// Lowercases the first ASCII character in a generated identifier fragment.
pub(crate) fn lower_first(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    format!(
        "{}{}",
        first.to_ascii_lowercase(),
        chars.collect::<String>()
    )
}

/// Uppercases the first ASCII character in a generated identifier fragment.
pub(crate) fn upper_first(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    format!(
        "{}{}",
        first.to_ascii_uppercase(),
        chars.collect::<String>()
    )
}
