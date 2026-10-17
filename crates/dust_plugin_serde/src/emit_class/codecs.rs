use super::*;

/// Renders the generated serializer support class for a class.
pub(crate) fn emit_serializer_support_type(class: &ClassIr, names: &JsonNames) -> String {
    let class_type = render_class_type(class);
    let arguments = render_type_arguments(&class.type_parameters);
    let json_type = names.json_type();
    let value = names.local("value");
    let method = format!(
        "  {json_type} serialize({class_type} {value}) => _${}Serialize{arguments}({value});",
        class.name
    );
    emit_codec(class, names, "Serializer", &method)
}

/// Renders the generated deserializer support class for a class.
pub(crate) fn emit_deserializer_support_type(class: &ClassIr, names: &JsonNames) -> String {
    let class_type = render_class_type(class);
    let arguments = render_type_arguments(&class.type_parameters);
    let json_type = names.json_type();
    let json = names.local("json");
    let method = format!(
        "  {class_type} deserialize({json_type} {json}) => _${}Deserialize{arguments}({json});",
        class.name
    );
    emit_codec(class, names, "Deserializer", &method)
}

/// Preserves the legacy declaration; only captured generic signatures need a private base.
fn emit_codec(class: &ClassIr, names: &JsonNames, direction: &str, method: &str) -> String {
    let class_type = render_class_type(class);
    let parameters = render_type_parameters(&class.type_parameters);
    let arguments = render_type_arguments(&class.type_parameters);
    let interface = names.reference(direction);
    let json_type = names.json_type();
    let public = format!("${}{}", class.name, direction);
    let generic = !class.type_parameters.is_empty();
    let docs = if generic {
        format!("/// Typed JSON {direction} for `{class_type}`.\n")
    } else {
        String::new()
    };
    let constructor_docs = if generic {
        "  /// Creates a typed JSON codec.\n"
    } else {
        ""
    };
    let mut facade = String::new();
    let (kind, implementation, implementation_docs, implementation_constructor_docs) = if let Some(
        base,
    ) =
        names.codec_base(direction)
    {
        facade = format!(
            "{docs}final class {public}{parameters} extends {base}{arguments} {{\n{constructor_docs}  const {public}();\n}}\n\n"
        );
        ("abstract", base, String::new(), "")
    } else {
        ("final", public.as_str(), docs, constructor_docs)
    };
    format!(
        r#"{facade}{implementation_docs}{kind} class {implementation}{parameters} implements {interface}<{class_type}, {json_type}> {{
{implementation_constructor_docs}  const {implementation}();

  @override
{method}
}}"#
    )
}
