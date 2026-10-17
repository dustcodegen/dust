use std::collections::{BTreeMap, BTreeSet};

use dust_dart_emit::{NameAllocator, library_reference_names};
use dust_ir::{BuiltinType, ClassIr, DartFileIr, TypeIr};
use heck::AsPascalCase;

/// Generated references and locals for one owning class, without rewriting its binders.
#[derive(Default)]
pub(crate) struct JsonNames {
    /// Core/runtime references protected from owning class binders.
    references: BTreeMap<&'static str, String>,
    /// Generated locals and private implementation names.
    locals: BTreeMap<String, String>,
}

impl JsonNames {
    /// Keeps legacy references unless this class captures a generated name.
    pub(crate) fn reference<'a>(&'a self, name: &'a str) -> &'a str {
        self.references
            .get(name)
            .map(String::as_str)
            .unwrap_or(name)
    }

    /// Keeps legacy locals unless a generic scope requires allocation.
    pub(crate) fn local<'a>(&'a self, name: &'a str) -> &'a str {
        self.locals.get(name).map(String::as_str).unwrap_or(name)
    }

    /// Renders the runtime JSON object type outside the model's binder meanings.
    pub(crate) fn json_type(&self) -> String {
        format!(
            "{}<{}, {}?>",
            self.reference("Map"),
            self.reference("String"),
            self.reference("Object")
        )
    }

    /// Renders map literal arguments with the same runtime JSON meanings.
    pub(crate) fn json_arguments(&self) -> String {
        format!(
            "<{}, {}?>",
            self.reference("String"),
            self.reference("Object")
        )
    }

    /// A private implementation keeps scoped aliases out of public method declarations.
    pub(crate) fn codec_base(&self, direction: &str) -> Option<&str> {
        self.locals.get(direction).map(String::as_str)
    }
}

/// Per-library aliases shared by generic SerDe classes.
pub(crate) struct JsonNamePlan {
    /// Names scoped to each declared or generated model.
    by_class: BTreeMap<String, JsonNames>,
    /// Alias declarations live outside class-parameter scopes.
    pub(crate) declarations: Vec<String>,
}

impl JsonNamePlan {
    /// Returns the generated names for a class present in the plan.
    pub(crate) fn names_for(&self, class: &ClassIr) -> &JsonNames {
        &self.by_class[&class.name]
    }
}

/// Reuses the shared allocator and already-lowered lexical declaration facts.
pub(crate) fn plan_json_names(library: &DartFileIr, generated: &[&ClassIr]) -> JsonNamePlan {
    let mut reserved = library_reference_names(library);
    reserved.extend(
        library
            .import_directives
            .iter()
            .filter_map(|import| import.prefix.clone()),
    );
    reserved.extend(generated.iter().map(|class| class.name.clone()));
    let mut allocator = NameAllocator::new(reserved);
    let mut aliases = BTreeMap::new();
    let mut plan = JsonNamePlan {
        by_class: BTreeMap::new(),
        declarations: vec![],
    };
    for class in library.classes.iter().chain(generated.iter().copied()) {
        let mut names = JsonNames::default();
        if !class.type_parameters.is_empty() {
            for name in runtime_references(class) {
                if !class.type_parameters.iter().any(|p| p.name.source == name)
                    && !class.fields.iter().any(|field| field.name == name)
                    && !class.methods.iter().any(|method| method.name == name)
                {
                    continue;
                }
                let alias = aliases.entry(name).or_insert_with(|| {
                    let alias = allocator.allocate(format!("_DustSerde{name}"));
                    let parameters =
                        if matches!(name, "Map" | "MapEntry" | "Serializer" | "Deserializer") {
                            let first = allocator.allocate("D");
                            let second = allocator.allocate("J");
                            format!("<{first}, {second}>")
                        } else {
                            String::new()
                        };
                    plan.declarations
                        .push(format!("typedef {alias}{parameters} = {name}{parameters};"));
                    alias
                });
                names.references.insert(name, alias.clone());
            }
            if ["Map", "String", "Object"]
                .iter()
                .any(|name| names.references.contains_key(name))
            {
                for direction in ["Serializer", "Deserializer"] {
                    let base = allocator.allocate(format!(
                        "_Dust{}{}",
                        class.name.trim_start_matches('_'),
                        direction
                    ));
                    names.locals.insert(direction.into(), base);
                }
            }
            let mut locals = allocator.clone();
            for name in [
                "instance",
                "json",
                "value",
                "item",
                "itemKey",
                "valueKey",
                "key",
                "allowedKeys",
            ] {
                names.locals.insert(name.into(), locals.allocate(name));
            }
            for field in &class.fields {
                for name in [
                    format!("{}Value", field.name),
                    format!("raw{}", AsPascalCase(&field.name)),
                    format!("raw{}Key", AsPascalCase(&field.name)),
                ] {
                    let allocated = locals.allocate(&name);
                    names.locals.insert(name, allocated);
                }
            }
        }
        plan.by_class.insert(class.name.clone(), names);
    }
    plan
}

/// Collects references generated by ordinary class helpers, not opaque user expressions.
fn runtime_references(class: &ClassIr) -> BTreeSet<&'static str> {
    let serialize = class
        .traits
        .iter()
        .any(|t| t.symbol.0 == "dust_dart::Serialize");
    let deserialize = class
        .traits
        .iter()
        .any(|t| t.symbol.0 == "dust_dart::Deserialize");
    let mut names = BTreeSet::new();
    if !serialize && !deserialize {
        return names;
    }
    names.extend(["Map", "String", "Object"]);
    if serialize {
        names.extend(["Serializable", "Serializer"]);
    }
    if deserialize {
        names.insert("Deserializer");
        if class
            .serde
            .as_ref()
            .is_some_and(|s| s.disallow_unrecognized_keys)
        {
            names.insert("ArgumentError");
        }
    }
    for field in &class.fields {
        let encode = serialize && !field.serde.as_ref().is_some_and(|s| s.skip_serializing);
        let decode = deserialize && !field.serde.as_ref().is_some_and(|s| s.skip_deserializing);
        if field
            .serde
            .as_ref()
            .is_some_and(|s| s.codec_source.is_some())
        {
            if encode || decode {
                names.insert("JsonHelper");
            }
        } else {
            if decode {
                names.insert("JsonHelper");
            }
            type_references(&field.ty, encode, decode, &mut names);
        }
    }
    names
}

/// Only generated conversion names are aliased; written field types retain their meaning.
fn type_references(ty: &TypeIr, encode: bool, decode: bool, names: &mut BTreeSet<&'static str>) {
    match ty {
        TypeIr::Builtin { kind, .. } if decode => {
            names.insert(if *kind == BuiltinType::Double {
                "num"
            } else {
                kind.as_str()
            });
        }
        TypeIr::Named { name, args, .. } if matches!(name.as_ref(), "List" | "Set" | "Map") => {
            if encode && name.as_ref() == "Map" {
                names.insert("MapEntry");
            }
            for arg in args.iter() {
                type_references(arg, encode, decode, names);
            }
        }
        _ => {}
    }
}
