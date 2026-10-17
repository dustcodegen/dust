use std::collections::HashSet;

use dust_ir::{NameIr, SpanIr, TypeIr};
use dust_parser_dart::ParsedDirective;

use crate::{ResolvedClass, types::lower_redirect_target_ir};

/// Distinguishes `Class.named` from `prefix.Class` without rewriting parser surfaces.
pub(crate) fn resolve_redirect_targets(
    classes: &mut [ResolvedClass],
    directives: &[ParsedDirective],
) {
    let prefixes = directives
        .iter()
        .filter_map(|directive| match directive {
            ParsedDirective::Import {
                prefix: Some(prefix),
                ..
            } => Some(prefix.as_str()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    for class in classes {
        let parameters = class
            .type_parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>();
        for constructor in &mut class.constructors {
            let Some(parsed) = &constructor.surface.redirected_target else {
                continue;
            };
            let mut target = lower_redirect_target_ir(class.span.file_id, parsed, &parameters);
            if target.constructor.is_none()
                && target.ty.args().is_empty()
                && let [owner, suffix] = parsed.type_name_parts.as_slice()
                && !prefixes.contains(owner.name.as_str())
            {
                // Without arguments, the grammar may classify `.named` as a type qualifier.
                target.ty = TypeIr::named(owner.name.as_str());
                target.constructor = Some(NameIr {
                    source: suffix.name.clone(),
                    short: suffix.name.clone(),
                    prefix: None,
                    span: SpanIr::new(class.span.file_id, suffix.span),
                });
            }
            constructor.redirected_target = Some(target);
        }
    }
}
