use dust_dart_emit::{
    DYNAMIC_TYPES, dart_string_literal, render_class_type, render_type_arguments,
    render_type_parameters,
};
use dust_ir::{ClassIr, SerdeClassConfigIr, SerdeVariantConfigIr};

use crate::{
    names::{JsonNamePlan, JsonNames},
    variant_types::{applied_arguments, variant_type},
};

/// Returns whether a class should use sealed dispatch helpers.
pub(crate) fn is_sealed_serde_class(class: &ClassIr) -> bool {
    sealed_serde(class).is_some()
}

/// Renders the top-level helper that serializes a sealed base instance.
pub(crate) fn emit_sealed_to_json_helper(
    class: &ClassIr,
    serializable_classes: &[&str],
    names: &JsonNames,
) -> Option<String> {
    let serde = sealed_serde(class)?;
    let parameters = render_type_parameters(&class.type_parameters);
    let arguments = render_type_arguments(&class.type_parameters);
    let class_type = render_class_type(class);
    let json_type = names.json_type();
    let instance = names.local("instance");
    let value = names.local("value");
    let mut lines = vec![
        format!(
            "{json_type} _${}Serialize{parameters}({class_type} {instance}) {{",
            class.name
        ),
        format!("  return switch ({instance}) {{"),
    ];
    for variant in &serde.variants {
        let target = DYNAMIC_TYPES.render(&variant_type(variant));
        let encoded = variant_to_json_expr(variant, value, serializable_classes);
        if serde.untagged {
            lines.push(format!("    {target} {value} => {encoded},"));
        } else {
            lines.push(format!(
                "    {target} {value} => {}{{",
                names.json_arguments()
            ));
            let key = dart_string_literal(serde.tag.as_deref()?);
            let tag = dart_string_literal(&variant.tag);
            if let Some(content) = &serde.content {
                lines.push(format!("      {key}: {tag},"));
                lines.push(format!(
                    "      {}: {encoded},",
                    dart_string_literal(content)
                ));
            } else {
                lines.push(format!("      ...{encoded},"));
                lines.push(format!("      {key}: {tag},"));
            }
            lines.push("    },".to_owned());
        }
    }
    if !class.type_parameters.is_empty() {
        // Dart cannot prove exhaustive coverage for dependent generic subtype patterns.
        lines.push(format!(
            "    _ => throw {}.value({instance}, 'instance', 'Unknown SerDe variant for {}'),",
            names.reference("ArgumentError"),
            class.name
        ));
    }
    lines.extend([
        "  };".into(),
        "}".into(),
        String::new(),
        format!(
            "{json_type} _${}ToJson{parameters}({class_type} {instance}) =>",
            class.name
        ),
        format!("    _${}Serialize{arguments}({instance});", class.name),
    ]);
    Some(lines.join("\n"))
}

/// Renders the top-level helper that deserializes a sealed base instance.
pub(crate) fn emit_sealed_from_json_helper(
    class: &ClassIr,
    deserializable_classes: &[&str],
    names: &JsonNames,
    plan: &JsonNamePlan,
) -> Option<String> {
    let serde = sealed_serde(class)?;
    let mut lines = from_json_header(class, names);
    let json = names.local("json");
    let error = names.reference("ArgumentError");
    if serde.untagged {
        for variant in &serde.variants {
            lines.push("  try {".into());
            lines.push(format!(
                "    return {};",
                variant_from_json_expr(
                    variant,
                    json,
                    deserializable_classes,
                    plan.variant_decoder(class, variant)
                )
            ));
            lines.push(format!("  }} on {} {{", names.reference("Object")));
            lines.push("    // Try the next untagged SerDe variant.".into());
            lines.push("  }".into());
        }
        lines.extend([
            format!("  throw {error}.value("),
            format!("    {json},"),
            "    'json',".into(),
            format!("    'no matching SerDe variant for {}',", class.name),
            "  );".into(),
        ]);
    } else {
        let tag = dart_string_literal(serde.tag.as_deref()?);
        let helper = names.reference("JsonHelper");
        let string = names.reference("String");
        let tag_value = names.local("tagValue");
        lines.push(format!(
            "  final {tag_value} = {helper}.as<{string}>({json}[{tag}], {tag}, 'String');"
        ));
        let variant_json = if let Some(content) = &serde.content {
            let key = dart_string_literal(content);
            let content_value = names.local("contentValue");
            lines.push(format!(
                "  final {content_value} = {helper}.asMap({json}[{key}], {key});"
            ));
            content_value
        } else {
            let variant_json = names.local("variantJson");
            lines.push(format!(
                "  final {variant_json} = {}.from({json})..remove({tag});",
                names.json_type()
            ));
            variant_json
        };
        lines.push(String::new());
        lines.push(format!("  return switch ({tag_value}) {{"));
        for variant in &serde.variants {
            lines.push(format!(
                "    {} => {},",
                dart_string_literal(&variant.tag),
                variant_from_json_expr(
                    variant,
                    variant_json,
                    deserializable_classes,
                    plan.variant_decoder(class, variant)
                )
            ));
        }
        lines.push(format!(
            "    _ => throw {error}('Unknown SerDe variant tag: ${tag_value}'),"
        ));
        lines.push("  };".into());
    }
    append_from_json_alias(&mut lines, class, names);
    Some(lines.join("\n"))
}

/// Starts a typed decode helper while preserving the legacy source hint.
fn from_json_header(class: &ClassIr, names: &JsonNames) -> Vec<String> {
    vec![
        format!(
            "// factory {}.fromJson(Map<String, Object?> json) => _${}FromJson(json);",
            class.name, class.name
        ),
        format!(
            "{} _${}Deserialize{}({} {}) {{",
            render_class_type(class),
            class.name,
            render_type_parameters(&class.type_parameters),
            names.json_type(),
            names.local("json")
        ),
    ]
}

/// Closes either decode representation with the same typed forwarding alias.
fn append_from_json_alias(lines: &mut Vec<String>, class: &ClassIr, names: &JsonNames) {
    lines.extend([
        "}".into(),
        String::new(),
        format!(
            "{} _${}FromJson{}({} {}) =>",
            render_class_type(class),
            class.name,
            render_type_parameters(&class.type_parameters),
            names.json_type(),
            names.local("json")
        ),
        format!(
            "    _${}Deserialize{}({});",
            class.name,
            render_type_arguments(&class.type_parameters),
            names.local("json")
        ),
    ]);
}

/// Renders one variant serialization expression.
fn variant_to_json_expr(variant: &SerdeVariantConfigIr, value: &str, classes: &[&str]) -> String {
    if classes.contains(&variant.target_class_name.as_str()) {
        format!(
            "_${}Serialize{}({value})",
            variant.target_class_name,
            applied_arguments(&variant_type(variant))
        )
    } else {
        format!("{value}.toJson()")
    }
}

/// Renders one variant deserialization expression.
fn variant_from_json_expr(
    variant: &SerdeVariantConfigIr,
    json: &str,
    classes: &[&str],
    selected: Option<&str>,
) -> String {
    let ty = variant_type(variant);
    if let Some(name) = selected {
        return format!("{name}{}({json})", applied_arguments(&ty));
    }
    if classes.contains(&variant.target_class_name.as_str()) {
        format!(
            "_${}Deserialize{}({json})",
            variant.target_class_name,
            applied_arguments(&ty)
        )
    } else {
        format!("{}.fromJson({json})", DYNAMIC_TYPES.render(&ty))
    }
}

/// Returns sealed serde metadata when generation is supported here.
fn sealed_serde(class: &ClassIr) -> Option<&SerdeClassConfigIr> {
    let serde = class.serde.as_ref()?;
    if serde.variants.is_empty() || (!serde.untagged && serde.tag.is_none()) {
        return None;
    }
    Some(serde)
}
