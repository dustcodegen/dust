use std::collections::{HashMap, HashSet};

use dust_dart_emit::DYNAMIC_TYPES;
use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::{ClassIr, DartFileIr, SerdeVariantConfigIr, TypeIr, TypeParamIr, substitute_type};

/// The type actually selected by a sealed factory, in the base's binder scope.
pub(crate) fn variant_type(variant: &SerdeVariantConfigIr) -> TypeIr {
    variant
        .target
        .as_ref()
        .map(|target| target.ty.clone())
        .unwrap_or_else(|| TypeIr::named(variant.target_class_name.as_str()))
}

/// Forwards structured use-site arguments without parsing rendered type source.
pub(crate) fn applied_arguments(ty: &TypeIr) -> String {
    if ty.args().is_empty() {
        String::new()
    } else {
        format!(
            "<{}>",
            ty.args()
                .iter()
                .map(|argument| DYNAMIC_TYPES.render(argument))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// A declaration's own applied type, without replacing or renaming its binders.
pub(crate) fn class_type(class: &ClassIr) -> TypeIr {
    TypeIr::generic(
        class.name.as_str(),
        class
            .type_parameters
            .iter()
            .map(|parameter| TypeIr::named(parameter.name.source.as_str()))
            .collect(),
    )
}

/// Finds a proven local ancestor application in the child's owning parameter scope.
pub(crate) fn base_type(
    class: &ClassIr,
    base: &ClassIr,
    library: &DartFileIr,
) -> Result<TypeIr, &'static str> {
    let mut current = class;
    let mut applied = class_type(class);
    for _ in 0..=library.classes.len() {
        if current.type_parameters.len() != applied.args().len() {
            return Err("wrong superclass argument count");
        }
        if current.name == base.name {
            return Ok(applied);
        }
        let bindings = current
            .type_parameters
            .iter()
            .zip(applied.args())
            .map(|(parameter, argument)| (parameter.name.source.as_str(), argument))
            .collect::<HashMap<_, _>>();
        let superclass = match &current.superclass {
            Some(superclass) => superclass.ty.clone(),
            None if current.type_parameters.is_empty() => current
                .superclass_name
                .as_deref()
                .map(TypeIr::named)
                .ok_or("unproven local superclass")?,
            None => return Err("missing applied superclass facts"),
        };
        applied = substitute_type(&superclass, &bindings)?;
        let TypeIr::Named {
            name,
            nullable: false,
            ..
        } = &applied
        else {
            return Err("non-class superclass");
        };
        current = library
            .classes
            .iter()
            .find(|class| class.name == name.as_ref())
            .ok_or("unproven local superclass")?;
    }
    Err("cyclic superclass application")
}

/// Missing declarations can retain only a complete permutation of known source binders.
pub(crate) fn generated_parameters(
    base: &ClassIr,
    variant: &SerdeVariantConfigIr,
) -> Result<Vec<TypeParamIr>, &'static str> {
    let applied = variant_type(variant);
    if applied.args().len() != base.type_parameters.len() {
        return Err("generated target must carry every owning type parameter explicitly");
    }
    let mut seen = HashSet::new();
    applied
        .args()
        .iter()
        .map(|argument| {
            let TypeIr::Named {
                name,
                args,
                nullable: false,
            } = argument
            else {
                return Err("generated target arguments must be bare owning type parameters");
            };
            if !args.is_empty() {
                return Err("generated target arguments must be bare owning type parameters");
            }
            if !seen.insert(name.as_ref()) {
                return Err("generated target arguments must be distinct owning type parameters");
            }
            base.type_parameters
                .iter()
                .find(|parameter| parameter.name.source == name.as_ref())
                .cloned()
                .ok_or("generated target argument has no owning source binder")
        })
        .collect()
}

/// Rejects unproven applications before generation can replace good output or cache.
pub(crate) fn validate_variant_types(library: &DartFileIr) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for base in &library.classes {
        let Some(serde) = &base.serde else {
            continue;
        };
        for variant in &serde.variants {
            let outcome = validate_variant(base, variant, library);
            if let Err(reason) = outcome {
                let span = variant
                    .target
                    .as_ref()
                    .map_or(base.span, |target| target.span);
                diagnostics.push(
                    Diagnostic::error(format!(
                        "Dust cannot generate sealed variant `{}.{}`: {reason}",
                        base.name, variant.constructor_name
                    ))
                    .with_label(SourceLabel::new(
                        span.file_id,
                        span.range,
                        "unsupported sealed target application",
                    )),
                );
            }
        }
    }
    diagnostics
}

/// Requires declaration identity and scope-correct arguments, not source-string guesses.
fn validate_variant(
    base: &ClassIr,
    variant: &SerdeVariantConfigIr,
    library: &DartFileIr,
) -> Result<(), &'static str> {
    let applied = variant_type(variant);
    let TypeIr::Named {
        name,
        nullable: false,
        ..
    } = &applied
    else {
        return Err("redirect requires a non-nullable named class");
    };
    if name.as_ref() != variant.target_class_name || name.contains('.') {
        return Err("redirect target ownership is not proven in this library");
    }
    let Some(target) = library
        .classes
        .iter()
        .find(|class| class.name == variant.target_class_name)
    else {
        generated_parameters(base, variant)?;
        return Ok(());
    };
    if target.type_parameters.len() != applied.args().len() {
        return Err("wrong redirect target argument count");
    }
    let parent = base_type(target, base, library)?;
    let bindings = target
        .type_parameters
        .iter()
        .zip(applied.args())
        .map(|(parameter, argument)| (parameter.name.source.as_str(), argument))
        .collect::<HashMap<_, _>>();
    if substitute_type(&parent, &bindings)? != class_type(base) {
        return Err("target superclass arguments do not preserve the owning base application");
    }
    if variant.target.is_some()
        && crate::variant_decoders::wants_deserialize(base)
        && crate::variant_decoders::wants_deserialize(target)
    {
        let Some((_, constructor)) =
            crate::variant_decoders::redirect_constructor(library, variant)
        else {
            return Err("redirect constructor is not declared on the target class");
        };
        if !constructor.can_construct_all_fields(&target.fields)
            || constructor.params.iter().any(|parameter| {
                !parameter.has_default
                    && !target
                        .fields
                        .iter()
                        .any(|field| field.name == parameter.name)
            })
        {
            return Err("redirect constructor cannot initialize every decoded variant field");
        }
    }
    Ok(())
}
