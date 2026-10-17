use dust_parser_dart::{ParsedAnnotation, ParsedDartFileSurface, ParsedDirective};

/// Discovery visits represented metadata sites, not source-string annotation names.
pub(super) fn annotations(parsed: &ParsedDartFileSurface) -> Vec<&ParsedAnnotation> {
    let mut annotations = Vec::new();
    for directive in &parsed.directives {
        if let ParsedDirective::Library {
            annotations: values,
            ..
        } = directive
        {
            annotations.extend(values);
        }
    }
    for class in &parsed.classes {
        annotations.extend(&class.annotations);
        for field in &class.fields {
            annotations.extend(&field.annotations);
        }
        for constructor in &class.constructors {
            annotations.extend(&constructor.annotations);
            for parameter in &constructor.params {
                annotations.extend(&parameter.annotations);
                if let Some(formal) = &parameter.function_formal {
                    annotations.extend(formal.annotations());
                }
            }
        }
        for method in &class.methods {
            annotations.extend(&method.annotations);
            for parameter in &method.params {
                annotations.extend(&parameter.annotations);
                if let Some(formal) = &parameter.function_formal {
                    annotations.extend(formal.annotations());
                }
            }
        }
    }
    for item in &parsed.enums {
        annotations.extend(&item.annotations);
        for variant in &item.variants {
            annotations.extend(&variant.annotations);
        }
    }
    for item in &parsed.mixins {
        annotations.extend(&item.annotations);
        for field in &item.fields {
            annotations.extend(&field.annotations);
        }
    }
    for item in &parsed.extensions {
        annotations.extend(&item.annotations);
    }
    for item in &parsed.extension_types {
        annotations.extend(&item.annotations);
    }
    for item in &parsed.typedefs {
        annotations.extend(&item.annotations);
    }
    for item in &parsed.variables {
        annotations.extend(&item.annotations);
    }
    for item in &parsed.functions {
        annotations.extend(&item.annotations);
        for parameter in &item.params {
            annotations.extend(&parameter.annotations);
            if let Some(formal) = &parameter.function_formal {
                annotations.extend(formal.annotations());
            }
        }
    }
    annotations
}
