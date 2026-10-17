use std::collections::HashSet;

use crate::{
    names::JsonNames,
    variant_types::{class_type, generated_parameters},
};
use dust_dart_emit::{DYNAMIC_TYPES, render_type_arguments, render_type_parameters};
use dust_ir::{
    ClassIr, ClassKindIr, ConstructorIr, ConstructorParamIr, DartFileIr, FieldIr, ParamKind,
    SerdeClassConfigIr, SerdeFieldConfigIr, SuperclassIr, TypeIr,
};

/// A concrete sealed variant class generated from a redirecting factory.
pub(crate) struct GeneratedVariantClass {
    /// The synthesized class model used by existing SerDe helpers.
    pub(crate) class: ClassIr,
    /// The sealed base class this variant extends.
    pub(crate) base_type: TypeIr,
    /// Whether to emit serialization helpers for this variant.
    pub(crate) serializable: bool,
    /// Whether to emit deserialization helpers for this variant.
    pub(crate) deserializable: bool,
}

/// Returns variant classes missing from user source and therefore generated.
pub(crate) fn generated_variant_classes(library: &DartFileIr) -> Vec<GeneratedVariantClass> {
    let existing = library
        .classes
        .iter()
        .map(|class| class.name.as_str())
        .collect::<HashSet<_>>();
    let mut generated = Vec::new();

    for base in &library.classes {
        let Some(serde) = &base.serde else {
            continue;
        };
        for variant in &serde.variants {
            if existing.contains(variant.target_class_name.as_str()) {
                continue;
            }
            let Ok(parameters) = generated_parameters(base, variant) else {
                continue;
            };
            let mut class = synthesize_class(base, serde, variant);
            class.type_parameters = parameters;
            generated.push(GeneratedVariantClass {
                class,
                base_type: class_type(base),
                serializable: wants_trait(base, "dust_dart::Serialize"),
                deserializable: wants_trait(base, "dust_dart::Deserialize"),
            });
        }
    }

    generated
}

/// Renders the Dart source for one generated concrete variant class.
pub(crate) fn emit_generated_variant_class(
    variant: &GeneratedVariantClass,
    names: &JsonNames,
) -> String {
    let class = &variant.class;
    let generic = !class.type_parameters.is_empty();
    let mut lines = Vec::new();
    if generic {
        lines.push(format!(
            "/// A generated variant of `{}`.",
            DYNAMIC_TYPES.render(&variant.base_type)
        ));
    }
    lines.push(format!(
        "final class {}{} extends {} {{",
        class.name,
        render_type_parameters(&class.type_parameters),
        DYNAMIC_TYPES.render(&variant.base_type)
    ));
    if generic {
        lines.push("  /// Creates this sealed variant.".into());
    }
    lines.extend(render_constructor(class));
    if variant.deserializable {
        lines.push(String::new());
        if generic {
            lines.push("  /// Decodes this variant's payload.".into());
        }
        lines.push(format!(
            "  factory {}.fromJson({} {}) =>",
            class.name,
            names.factory_json_type(),
            names.local("json")
        ));
        lines.push(format!(
            "      _${}FromJson{}({});",
            class.name,
            render_type_arguments(&class.type_parameters),
            names.local("json")
        ));
    }
    if !class.fields.is_empty() {
        lines.push(String::new());
    }
    for field in &class.fields {
        if generic {
            lines.push(format!("  /// The `{}` payload value.", field.name));
        }
        lines.push(format!(
            "  final {} {};",
            DYNAMIC_TYPES.render(&field.ty),
            field.name
        ));
    }
    lines.push("}".to_owned());
    lines.join("\n")
}

/// Builds a class model for an omitted redirect target.
fn synthesize_class(
    base: &ClassIr,
    base_serde: &SerdeClassConfigIr,
    variant: &dust_ir::SerdeVariantConfigIr,
) -> ClassIr {
    ClassIr {
        type_parameters: base.type_parameters.clone(),
        mixins: Vec::new(),
        kind: ClassKindIr::Class,
        name: variant.target_class_name.clone(),
        is_abstract: false,
        is_interface: false,
        superclass_name: Some(base.name.clone()),
        superclass: Some(SuperclassIr {
            ty: class_type(base),
            span: base.span,
        }),
        span: base.span,
        fields: variant
            .params
            .iter()
            .map(|param| {
                let mut field = field_from_param(param);
                if let Some(serde) = variant.field_serde.get(&param.name) {
                    let default = field
                        .serde
                        .as_ref()
                        .and_then(|serde| serde.default_value_source.clone());
                    let mut serde = serde.clone();
                    if serde.default_value_source.is_none() {
                        serde.default_value_source = default;
                    }
                    field.serde = Some(serde);
                }
                field
            })
            .collect(),
        constructors: vec![ConstructorIr {
            name: variant
                .target
                .as_ref()
                .and_then(|target| target.constructor.as_ref())
                .filter(|name| name.source != "new")
                .map(|name| name.source.clone()),
            is_factory: false,
            redirected_target_source: None,
            redirected_target_name: None,
            redirected_target: None,
            span: base.span,
            params: variant.params.clone(),
        }],
        methods: Vec::new(),
        traits: base.traits.clone(),
        configs: Vec::new(),
        serde: Some(SerdeClassConfigIr {
            rename_all: base_serde.rename_all,
            disallow_unrecognized_keys: base_serde.disallow_unrecognized_keys,
            ..SerdeClassConfigIr::default()
        }),
    }
}

/// Converts one factory parameter into a generated final field.
fn field_from_param(param: &ConstructorParamIr) -> FieldIr {
    FieldIr {
        name: param.name.clone(),
        ty: param.ty.clone(),
        span: param.span,
        has_default: false,
        serde: param
            .default_value_source
            .as_ref()
            .map(|default| SerdeFieldConfigIr {
                default_value_source: Some(default.clone()),
                ..SerdeFieldConfigIr::default()
            }),
        configs: Vec::new(),
    }
}

/// Renders the generated const constructor.
fn render_constructor(class: &ClassIr) -> Vec<String> {
    let constructor_name = class
        .constructors
        .first()
        .and_then(|constructor| constructor.name.as_deref())
        .map_or_else(
            || class.name.clone(),
            |name| format!("{}.{name}", class.name),
        );
    let params = class
        .constructors
        .first()
        .map(|constructor| constructor.params.as_slice())
        .unwrap_or(&[]);
    if params.is_empty() {
        return vec![format!("  const {constructor_name}() : super();")];
    }

    let positional = params
        .iter()
        .filter(|param| param.kind == ParamKind::Positional && param.is_required)
        .map(|param| render_param(param, 4))
        .collect::<Vec<_>>();
    let optional_params = params
        .iter()
        .filter(|param| param.kind == ParamKind::Positional && !param.is_required)
        .collect::<Vec<_>>();
    let named_params = params
        .iter()
        .filter(|param| param.kind == ParamKind::Named)
        .collect::<Vec<_>>();

    if positional.is_empty() {
        let (open, close, grouped) = if named_params.is_empty() {
            ("[", "]", &optional_params)
        } else {
            ("{", "}", &named_params)
        };
        let mut lines = vec![format!("  const {constructor_name}({open}")];
        lines.extend(grouped.iter().map(|param| render_param(param, 4)));
        lines.push(format!("  {close}) : super();"));
        return lines;
    }

    let mut lines = vec![format!("  const {constructor_name}(")];
    lines.extend(positional);
    if !optional_params.is_empty() {
        lines.push("    [".to_owned());
        lines.extend(optional_params.iter().map(|param| render_param(param, 6)));
        lines.push("    ]".to_owned());
    }
    if !named_params.is_empty() {
        lines.push("    {".to_owned());
        lines.extend(named_params.iter().map(|param| render_param(param, 6)));
        lines.push("    }".to_owned());
    }
    lines.push("  ) : super();".to_owned());
    lines
}

/// Renders one field formal without conflating nullability and presence.
fn render_param(param: &ConstructorParamIr, indent: usize) -> String {
    let prefix = if param.kind == ParamKind::Named && param.is_required {
        "required "
    } else {
        ""
    };
    let padding = " ".repeat(indent);
    match param.default_value_source.as_deref() {
        Some(default) => format!("{padding}{prefix}this.{} = {default},", param.name),
        None => format!("{padding}{prefix}this.{},", param.name),
    }
}

/// Returns whether a class has a resolved trait.
fn wants_trait(class: &ClassIr, symbol: &str) -> bool {
    class.traits.iter().any(|item| item.symbol.0 == symbol)
}
