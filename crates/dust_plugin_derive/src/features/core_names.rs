use std::collections::BTreeMap;

use dust_ir::{ClassIr, DartFileIr, TypeIr};

use super::{
    COPY_WITH_SYMBOL, DEBUG_SYMBOL, EQ_SYMBOL, TO_STRING_SYMBOL,
    eq_hash::has_trait,
    names::{NameAllocator, library_reference_names},
    validate::runtime_references,
    writer::find_clone_constructor,
};

/// Generated core/runtime references protected from the owning class's lexical names.
#[derive(Clone, Default)]
pub(crate) struct CoreNames {
    /// Only references requiring a private library alias.
    aliases: BTreeMap<&'static str, String>,
}

impl CoreNames {
    /// Keeps ordinary core references unchanged unless the class shadows them.
    pub(crate) fn get(&self, name: &'static str) -> &str {
        self.aliases.get(name).map(String::as_str).unwrap_or(name)
    }
}

/// Per-library aliases and per-class core reference scopes.
pub(crate) struct CoreNamePlan {
    /// Each emitted class has its own lexical scope.
    by_class: BTreeMap<String, CoreNames>,
    /// Shared private aliases, emitted only when referenced.
    pub(crate) declarations: Vec<String>,
}

impl CoreNamePlan {
    /// Returns the scope planned for this library class.
    pub(crate) fn names_for(&self, class: &ClassIr) -> &CoreNames {
        &self.by_class[&class.name]
    }
}

/// Plans core aliases without changing source binder or opaque type text.
pub(crate) fn plan_core_names(library: &DartFileIr, emit_form_helpers: bool) -> CoreNamePlan {
    let mut allocator = NameAllocator::new(library_reference_names(library));
    let mut shared = BTreeMap::new();
    let mut plan = CoreNamePlan {
        by_class: BTreeMap::new(),
        declarations: Vec::new(),
    };
    for class in &library.classes {
        let eq = has_trait(class, EQ_SYMBOL);
        let copy = has_trait(class, COPY_WITH_SYMBOL) && find_clone_constructor(class).is_some();
        let debug = has_trait(class, TO_STRING_SYMBOL) || has_trait(class, DEBUG_SYMBOL);
        let copy_sentinel = copy
            && class.fields.iter().any(|field| {
                !class.type_parameters.is_empty()
                    || field.ty.is_nullable()
                    || matches!(field.ty, TypeIr::Dynamic | TypeIr::Unknown)
            });
        let mut names = CoreNames::default();
        for (name, base, needed) in [
            (
                "Object",
                "_DustCoreObject",
                eq || (copy && !class.fields.is_empty()),
            ),
            ("String", "_DustCoreString", debug),
            ("bool", "_DustCoreBool", eq),
            ("int", "_DustCoreInt", eq),
            ("identical", "_dustCoreIdentical", eq || copy_sentinel),
            ("pragma", "_DustCorePragma", copy),
        ]
        .into_iter()
        .map(|(name, base, needed)| (name, base.to_owned(), needed))
        .chain(
            runtime_references(library, class, emit_form_helpers)
                .into_iter()
                .map(|name| {
                    let base = match name {
                        "String" | "List" | "RegExp" => format!("_DustCore{name}"),
                        _ => format!("_DustRuntime{name}"),
                    };
                    (name, base, true)
                }),
        ) {
            let shadowed = class.type_parameters.iter().any(|p| p.name.source == name)
                || class.fields.iter().any(|field| field.name == name)
                || class.methods.iter().any(|method| method.name == name);
            if !needed || !shadowed {
                continue;
            }
            let alias = shared.entry(name).or_insert_with(|| {
                let alias = allocator.allocate(&base);
                plan.declarations.push(if name == "identical" {
                    format!("const bool Function(Object?, Object?) {alias} = identical;")
                } else if name == "List" {
                    let item = allocator.allocate("E");
                    format!("typedef {alias}<{item}> = List<{item}>;")
                } else {
                    format!("typedef {alias} = {name};")
                });
                alias
            });
            names.aliases.insert(name, alias.clone());
        }
        plan.by_class.insert(class.name.clone(), names);
    }
    plan
}

/// A class binder can shadow Object's inherited getter in an unqualified expression.
pub(crate) fn runtime_type(class: &ClassIr) -> &'static str {
    if class
        .type_parameters
        .iter()
        .any(|p| p.name.source == "runtimeType")
    {
        "this.runtimeType"
    } else {
        "runtimeType"
    }
}
