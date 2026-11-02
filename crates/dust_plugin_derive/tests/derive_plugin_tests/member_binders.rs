use dust_ir::{NameIr, TypeParamIr};
use dust_plugin_api::DustPlugin;
use dust_plugin_derive::register_plugin;

use crate::support::{sample_library, span};

#[test]
fn rejects_only_binders_owned_by_requested_members() {
    let mut library = sample_library(&[
        "dust_dart::CopyWith",
        "dust_dart::ToString",
        "dust_dart::Eq",
        "dust_dart::Validate",
    ]);
    library.classes[0].type_parameters =
        ["copyWith", "toString", "hashCode", "validate", "unrelated"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| parameter(name, index as u32))
            .collect();

    let diagnostics = register_plugin().validate(&library);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        [
            "generic parameter `toString` on `User` conflicts with generated `toString` member",
            "generic parameter `hashCode` on `User` conflicts with generated `hashCode` member",
            "generic parameter `copyWith` on `User` conflicts with generated `copyWith` member",
            "generic parameter `validate` on `User` conflicts with generated `validate` member",
        ]
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.labels[0].range.start().to_u32())
            .collect::<Vec<_>>(),
        [11, 12, 10, 13]
    );
}

#[test]
fn allows_member_names_for_traits_that_do_not_generate_them() {
    let mut library = sample_library(&["dust_dart::Eq"]);
    library.classes[0].type_parameters = ["copyWith", "toString", "validate"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| parameter(name, index as u32))
        .collect();
    assert_eq!(register_plugin().validate(&library), []);
}

fn parameter(name: &str, offset: u32) -> TypeParamIr {
    TypeParamIr {
        name: NameIr {
            source: name.to_owned(),
            short: name.to_owned(),
            prefix: None,
            span: span(10 + offset, 11 + offset),
        },
        bound: None,
        span: span(10 + offset, 11 + offset),
    }
}
