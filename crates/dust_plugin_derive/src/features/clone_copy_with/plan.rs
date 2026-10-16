use std::collections::BTreeMap;

use dust_dart_emit::named_argument_name;
use dust_ir::DartFileIr;

use crate::features::{
    COPY_WITH_SYMBOL,
    eq_hash::has_trait,
    names::{NameAllocator, library_declaration_names, lower_first},
    writer::find_clone_constructor,
};

use super::{
    CopyWithNames, CopyWithPlan, CopyWithSample, CopyWithSentinel, ValueSampleKind,
    nested_target_type, render::render_sentinel_helper, sample_replacement_field, sample_value,
    support::needs_copy_with_sentinel,
};

/// Plans copyWith support names and shared helpers for the whole library.
pub(crate) fn plan_copy_with(library: &DartFileIr) -> CopyWithPlan {
    let declarations = library_declaration_names(library);
    let mut allocator = NameAllocator::new(
        declarations.iter().map(String::as_str).chain(
            library
                .classes
                .iter()
                .flat_map(|class| class.type_parameters.iter().map(|p| p.name.source.as_str())),
        ),
    );
    let mut by_class = BTreeMap::new();
    let mut samples_by_class = BTreeMap::new();
    let mut shared_helpers = Vec::new();
    let mut accepts_null_helper: Option<String> = None;

    for class in &library.classes {
        if !has_trait(class, COPY_WITH_SYMBOL) || find_clone_constructor(class).is_none() {
            continue;
        }
        let interface_name = allocator.allocate(format!("_${}CopyWith", class.name));
        let impl_name = allocator.allocate(format!("_${}CopyWithImpl", class.name));
        let mut binders = NameAllocator::new(
            declarations
                .iter()
                .map(String::as_str)
                .chain(class.type_parameters.iter().map(|p| p.name.source.as_str())),
        );
        let result_type = binders.allocate("$Res");
        let mut local_allocator = NameAllocator::new(
            class
                .fields
                .iter()
                .flat_map(|field| [field.name.as_str(), named_argument_name(&field.name)])
                .chain(class.type_parameters.iter().map(|p| p.name.source.as_str())),
        );
        let self_name = local_allocator.allocate("_self");
        let then_name = local_allocator.allocate("_then");
        let callback_value_name = local_allocator.allocate("value");
        let nested_value_names = class
            .fields
            .iter()
            .filter(|field| {
                nested_target_type(class, &field.ty).is_some_and(|(_, nullable)| nullable)
            })
            .map(|field| {
                (
                    field.name.clone(),
                    local_allocator.allocate(format!("{}Value", field.name)),
                )
            })
            .collect();

        if let Some(field) = sample_replacement_field(class) {
            samples_by_class.insert(
                class.name.clone(),
                CopyWithSample {
                    field_name: named_argument_name(&field.name).to_owned(),
                    nested_value: sample_value(&field.ty, ValueSampleKind::Nested),
                },
            );
        }
        let null_helper = if !class.type_parameters.is_empty() && !class.fields.is_empty() {
            Some(
                accepts_null_helper
                    .get_or_insert_with(|| {
                        let name = allocator.allocate("_copyWithAcceptsNull");
                        shared_helpers.push(format!("bool {name}<T>() => null is T;"));
                        name
                    })
                    .clone(),
            )
        } else {
            None
        };
        let sentinel = if class
            .fields
            .iter()
            .any(|field| null_helper.is_some() || needs_copy_with_sentinel(&field.ty))
        {
            let name = allocator.allocate(format!("_{}CopyWithUnset", class.name));
            let value_name =
                allocator.allocate(format!("_{}CopyWithUnset", lower_first(&class.name)));
            shared_helpers.push(render_sentinel_helper(&name, &value_name));
            Some(CopyWithSentinel { value_name })
        } else {
            None
        };
        by_class.insert(
            class.name.clone(),
            CopyWithNames {
                interface_name,
                impl_name,
                result_type,
                type_parameter_count: class.type_parameters.len(),
                accepts_null_helper: null_helper,
                sentinel,
                self_name,
                then_name,
                callback_value_name,
                nested_value_names,
            },
        );
    }
    CopyWithPlan {
        by_class,
        samples_by_class,
        shared_helpers,
    }
}
