use std::collections::BTreeSet;

use dust_diagnostics::Diagnostic;
use dust_ir::{ClassIr, DartFileIr, TypeIr};

use super::super::{Callable, library::top_level_declaration_span, values::error};

/// Rejects source names that a part file cannot safely requalify or import anew.
pub(super) fn validate(
    file: &DartFileIr,
    callable: Callable<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let controller = controller(file, callable);
    let ws_shadow = top_level_declaration_span(file, "ws")
        .or_else(|| {
            file.import_directives
                .iter()
                .find(|import| import.prefix.as_deref() == Some("ws"))
                .map(|import| import.span)
        })
        .or_else(|| {
            controller.and_then(|class| {
                class
                    .fields
                    .iter()
                    .find(|field| field.name == "ws")
                    .map(|field| field.span)
                    .or_else(|| {
                        class
                            .methods
                            .iter()
                            .find(|method| method.name == "ws")
                            .map(|method| method.span)
                    })
            })
        });
    if let Some(span) = ws_shadow {
        diagnostics.push(error(
            span,
            "declaration or import prefix `ws` shadows the WebSocket upgrade function",
            "rename the declaration or use a different import prefix, such as `socket`",
        ));
    }
    let mut reserved = BTreeSet::from(["request".to_owned()]);
    for index in 0..callable.params().len() {
        reserved.insert(format!("_dustValue{index}"));
        reserved.insert(format!("_dustResult{index}"));
    }
    for param in callable.params() {
        let mut roots = BTreeSet::new();
        collect_roots(&param.ty, &mut roots);
        for root in roots.intersection(&reserved) {
            diagnostics.push(error(param.span,
                format!("WebSocket type name or prefix `{root}` conflicts with a generated handler local"),
                "rename the import prefix or model type",
            ));
        }
    }
    if matches!(callable, Callable::Function(_)) && reserved.contains(callable.name()) {
        diagnostics.push(error(
            callable.span(),
            "WebSocket callable name conflicts with a generated handler local",
            "rename the top-level handler",
        ));
    }
    if let Some(class) = controller.filter(|class| reserved.contains(&class.name)) {
        diagnostics.push(error(
            class.span,
            "WebSocket controller name conflicts with a generated handler local",
            "rename the controller class",
        ));
    }
}

/// Finds the actual owning class without treating equal spans as identity.
fn controller<'a>(file: &'a DartFileIr, callable: Callable<'_>) -> Option<&'a ClassIr> {
    let Callable::Method(target) = callable else {
        return None;
    };
    file.classes.iter().find(|class| {
        class
            .methods
            .iter()
            .any(|method| std::ptr::eq(method, target))
    })
}

/// Collects structured type roots, including generic message model arguments.
fn collect_roots(ty: &TypeIr, roots: &mut BTreeSet<String>) {
    if let Some(name) = ty.name() {
        roots.insert(name.split('.').next().unwrap_or(name).to_owned());
    }
    for argument in ty.args() {
        collect_roots(argument, roots);
    }
}
