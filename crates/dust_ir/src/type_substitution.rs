use std::collections::HashMap;

use crate::TypeIr;

/// Replaces bare owning binders simultaneously; opaque source is never rewritten.
pub fn substitute_type(
    ty: &TypeIr,
    bindings: &HashMap<&str, &TypeIr>,
) -> Result<TypeIr, &'static str> {
    if bindings.is_empty() {
        return Ok(ty.clone());
    }
    match ty {
        TypeIr::Named {
            name,
            args,
            nullable,
        } => {
            if let Some(argument) = bindings.get(name.as_ref()) {
                if !args.is_empty() {
                    return Err("applied type parameter");
                }
                return Ok(if *nullable {
                    (*argument).clone().nullable()
                } else {
                    (*argument).clone()
                });
            }
            let args = args
                .iter()
                .map(|arg| substitute_type(arg, bindings))
                .collect::<Result<Vec<_>, _>>()?;
            let result = TypeIr::generic(name.clone(), args);
            Ok(if *nullable { result.nullable() } else { result })
        }
        TypeIr::Function { .. } => Err("opaque function type"),
        TypeIr::Record { .. } => Err("opaque record type"),
        TypeIr::Unknown => Err("unresolved type"),
        TypeIr::Builtin { .. } | TypeIr::Dynamic => Ok(ty.clone()),
    }
}
