use super::*;
use dust_parser_dart::ParsedFunctionFormalTypeSurface;

/// Visits owned parameter containers, never callback bodies or default values.
pub(super) fn extract(
    node: Node<'_>,
    parent: usize,
    source: &SourceText,
    scopes: &mut Vec<ParsedLexicalScope>,
) {
    if matches!(node.kind(), "formal_parameter" | "default_formal_parameter") {
        if let Some(formal) = crate::classes::function_formals::extract(node, source) {
            signature(&formal, parent, scopes);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor).filter(|child| {
        matches!(
            child.kind(),
            "formal_parameter_list"
                | "optional_formal_parameters"
                | "formal_parameter"
                | "default_formal_parameter"
        )
    }) {
        extract(child, parent, source, scopes);
    }
}

/// A callback's type binders apply inside its signature, not to outer metadata.
fn signature(
    formal: &ParsedFunctionFormalTypeSurface,
    parent: usize,
    scopes: &mut Vec<ParsedLexicalScope>,
) {
    let parent = if formal.type_parameters.is_empty() {
        parent
    } else {
        let index = scopes.len();
        scopes.push(ParsedLexicalScope {
            parent: Some(parent),
            name: Some(formal.name.clone()),
            kind: Scope::Callable,
            visibility: TextRange::new(
                formal
                    .return_type
                    .as_ref()
                    .map_or(formal.name.span.start(), |ty| ty.span.start()),
                formal.span.end(),
            ),
            span: formal.span,
            declarations: formal
                .type_parameters
                .iter()
                .map(|parameter| ParsedLexicalDeclaration {
                    name: ParsedLexicalName {
                        name: parameter.name.clone(),
                        span: parameter.name_span,
                    },
                    kind: Declaration::TypeParameter,
                    span: parameter.span,
                })
                .collect(),
        });
        index
    };
    for parameter in &formal.parameters {
        if let Some(nested) = &parameter.function_formal {
            signature(nested, parent, scopes);
        }
    }
}
