//! Normalizing the serde configuration on one field, and on a sealed hierarchy.

use super::*;

/// Normalizes field-level SerDe configuration after symbol resolution.
pub(crate) fn normalize_field_serde(
    field_name: &str,
    configs: &[ConfigApplicationIr],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SerdeFieldConfigIr> {
    let mut serde = SerdeFieldConfigIr::default();
    let mut saw_serde = false;
    for config in configs
        .iter()
        .filter(|config| config.symbol.0 == "dust_dart::SerDe")
    {
        saw_serde = true;
        for key in config.named_args.keys().map(String::as_str) {
            let source = config.named_argument_source(key).unwrap_or_default();
            match key {
                "rename" => match config.named_string(key) {
                    Some(value) => serde.rename = Some(value),
                    None => diagnostics.push(Diagnostic::error(format!("field `{field_name}` uses a non-string `SerDe(rename: ...)` value"))),
                },
                "aliases" => match config.named_string_list(key) {
                    Some(value) => serde.aliases = value,
                    None => diagnostics.push(Diagnostic::error(format!("field `{field_name}` uses a non-string-list `SerDe(aliases: ...)` value"))),
                },
                "using" => match config.named_argument_value(key) {
                    Some(AnnotationValueIr::Constructor { .. }) => serde.codec_source = Some(source.trim().to_owned()),
                    Some(AnnotationValueIr::Member(name)) if name.prefix.is_some() || name.source.contains('.') || name.short.chars().next().is_some_and(|first| first.is_ascii_lowercase()) => serde.codec_source = Some(source.trim().to_owned()),
                    Some(AnnotationValueIr::Member(_)) => diagnostics.push(Diagnostic::error(format!("field `{field_name}` uses suspicious `SerDe(using: ...)` type reference `{}`", source.trim())).with_note(codec_guidance())),
                    _ => diagnostics.push(Diagnostic::error(format!("field `{field_name}` uses invalid `SerDe(using: ...)` value `{}`", source.trim())).with_note(codec_guidance())),
                },
                "defaultValue" => {
                    serde.default_value_source = Some(source.trim().to_owned());
                    serde.default_value = config.named_argument_value(key).cloned();
                }
                "skip" => match config.named_bool(key) {
                    Some(true) => { serde.skip_serializing = true; serde.skip_deserializing = true; }
                    Some(false) => {}
                    None => diagnostics.push(Diagnostic::error(format!("field `{field_name}` uses a non-boolean `SerDe(skip: ...)` value"))),
                },
                "skipSerializing" => match config.named_bool(key) {
                    Some(value) => serde.skip_serializing = value,
                    None => diagnostics.push(Diagnostic::error(format!("field `{field_name}` uses a non-boolean `SerDe(skipSerializing: ...)` value"))),
                },
                "skipDeserializing" => match config.named_bool(key) {
                    Some(value) => serde.skip_deserializing = value,
                    None => diagnostics.push(Diagnostic::error(format!("field `{field_name}` uses a non-boolean `SerDe(skipDeserializing: ...)` value"))),
                },
                unsupported @ ("renameAll" | "tag" | "content" | "untagged" | "disallowUnrecognizedKeys") => diagnostics.push(Diagnostic::error(format!("field `{field_name}` does not support `SerDe({unsupported}: ...)`"))),
                unknown => diagnostics.push(Diagnostic::warning(format!("field `{field_name}` uses unknown `SerDe` option `{unknown}`"))),
            }
        }
    }
    saw_serde.then_some(serde)
}

/// Returns guidance for passing a codec instance instead of a type or literal.
pub(super) fn codec_guidance() -> &'static str {
    "Use a codec object such as `const UnixEpochDateTimeCodec()` or `unixEpochDateTimeCodec`."
}

/// Completes sealed class SerDe variant metadata from redirecting factory constructors.
pub(crate) fn normalize_sealed_serde_variants(
    classes: &mut [ResolvedClass],
    context: &crate::BindingContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let index_by_name = classes
        .iter()
        .enumerate()
        .map(|(index, class)| (class.name.clone(), index))
        .collect::<HashMap<_, _>>();
    for index in 0..classes.len() {
        let has_serde = classes[index].serde.is_some();
        let uses_sealed_serde = classes[index]
            .serde
            .as_ref()
            .is_some_and(|serde| serde.uses_sealed_representation());
        if !has_serde {
            continue;
        }

        let base_name = classes[index].name.clone();
        if classes[index].kind != ClassKindIr::SealedClass {
            if !uses_sealed_serde {
                continue;
            }
            diagnostics.push(Diagnostic::error(format!(
                "SerDe class `{base_name}` uses sealed variant options but is not sealed"
            )));
            continue;
        }

        let rename_all = classes[index]
            .serde
            .as_ref()
            .and_then(|serde| serde.rename_all);
        let constructors = &classes[index].constructors;
        let type_parameters = classes[index]
            .type_parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>();
        let mut variants = Vec::new();
        let mut seen_tags = HashSet::new();

        for constructor in constructors
            .iter()
            .filter(|constructor| constructor.surface.is_factory)
        {
            let Some(constructor_name) = constructor.surface.name.as_deref() else {
                continue;
            };
            let target = constructor.redirected_target.clone();
            let target_name = match target.as_ref().map(|target| &target.ty) {
                Some(TypeIr::Named { name, .. }) => Some(name.as_ref()),
                Some(_) => {
                    let span = target
                        .as_ref()
                        .map_or(classes[index].span, |target| target.span);
                    diagnostics.push(
                        Diagnostic::error(format!(
                            "Sealed SerDe redirect on `{base_name}` requires a named target class"
                        ))
                        .with_label(dust_diagnostics::SourceLabel::new(
                            span.file_id,
                            span.range,
                            "unsupported sealed redirect target",
                        )),
                    );
                    continue;
                }
                None => constructor.surface.redirected_target_name.as_deref(),
            };
            let Some(target_class_name) = target_name else {
                continue;
            };

            let target_superclass = index_by_name
                .get(target_class_name)
                .and_then(|target_index| classes.get(*target_index))
                .and_then(|target| target.superclass_name.as_deref());
            if target_superclass.is_some()
                && !local_superclass_reaches(target_class_name, &base_name, classes, &index_by_name)
            {
                diagnostics.push(Diagnostic::error(format!(
                    "Variant target class {target_class_name} does not extend {base_name}"
                )));
            }

            let tag = variant_serde_tag(constructor_name, constructor, rename_all, diagnostics);
            if !seen_tags.insert(tag.clone()) {
                diagnostics.push(Diagnostic::error(format!(
                    "Duplicate SerDe variant tag: {tag}"
                )));
            }

            variants.push(SerdeVariantConfigIr {
                constructor_name: constructor_name.to_owned(),
                target_class_name: target_class_name.to_owned(),
                target: target.clone(),
                field_serde: variant_field_serde(
                    classes[index].span.file_id,
                    constructor,
                    context,
                    diagnostics,
                ),
                tag,
                params: variant_constructor_params(
                    classes[index].span.file_id,
                    constructor,
                    &type_parameters,
                    diagnostics,
                ),
            });
        }

        if variants.is_empty() {
            diagnostics.push(Diagnostic::error(format!(
                "Sealed SerDe class {base_name} has no factory variants"
            )));
        }

        if let Some(serde) = &mut classes[index].serde {
            serde.variants = variants;
        }
    }
}

/// Checks declaration identity only; argument substitution belongs to shared type IR.
fn local_superclass_reaches(
    target: &str,
    base: &str,
    classes: &[ResolvedClass],
    indices: &HashMap<String, usize>,
) -> bool {
    let mut current = target;
    for _ in 0..classes.len() {
        let Some(parent) = indices
            .get(current)
            .and_then(|index| classes[*index].superclass_name.as_deref())
        else {
            return false;
        };
        if parent == base {
            return true;
        }
        current = parent;
    }
    false
}
