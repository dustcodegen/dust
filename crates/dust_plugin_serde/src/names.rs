use std::collections::BTreeMap;

use dust_dart_emit::{NameAllocator, library_reference_names};
use dust_ir::{ClassIr, DartFileIr, SerdeVariantConfigIr};
use heck::AsPascalCase;

/// Runtime references required by ordinary and sealed JSON helpers.
mod references;
use references::runtime_references;

/// Generated references and locals for one owning class, without rewriting its binders.
#[derive(Default)]
pub(crate) struct JsonNames {
    /// Core/runtime references protected from owning class binders.
    references: BTreeMap<&'static str, String>,
    /// Generated locals and private implementation names.
    locals: BTreeMap<String, String>,
    /// Public generated factories cannot expose private core aliases.
    factory_json: Option<String>,
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

    /// Uses a public payload alias only when a generated factory captures core names.
    pub(crate) fn factory_json_type(&self) -> String {
        self.factory_json
            .clone()
            .unwrap_or_else(|| self.json_type())
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
    /// Extra decoder symbols keyed by their owning factory.
    variant_decoders: BTreeMap<(String, String), String>,
}

impl JsonNamePlan {
    /// Returns the generated names for a class present in the plan.
    pub(crate) fn names_for(&self, class: &ClassIr) -> &JsonNames {
        &self.by_class[&class.name]
    }

    /// Returns the optional payload helper for an explicit constructor selection.
    pub(crate) fn variant_decoder(
        &self,
        base: &ClassIr,
        variant: &SerdeVariantConfigIr,
    ) -> Option<&str> {
        self.variant_decoders
            .get(&(base.name.clone(), variant.constructor_name.clone()))
            .map(String::as_str)
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
        variant_decoders: BTreeMap::new(),
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
                if !class.name.starts_with('_')
                    && generated
                        .iter()
                        .any(|generated| generated.name == class.name)
                {
                    let alias = allocator.allocate(format!("${}Json", class.name));
                    plan.declarations.push(format!("/// JSON payload accepted by generated `{}.fromJson`.\ntypedef {alias} = Map<String, Object?>;", class.name));
                    names.factory_json = Some(alias);
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
            if class
                .serde
                .as_ref()
                .is_some_and(|serde| !serde.variants.is_empty())
            {
                for name in ["tagValue", "variantJson", "contentValue"] {
                    names.locals.insert(name.into(), locals.allocate(name));
                }
            }
        }
        plan.by_class.insert(class.name.clone(), names);
    }
    for base in &library.classes {
        let Some(serde) = &base.serde else {
            continue;
        };
        for variant in &serde.variants {
            if crate::variant_decoders::needs_selected_decoder(base, variant, library) {
                let name = allocator.allocate(format!(
                    "_dust{}{}Deserialize",
                    base.name.trim_start_matches('_'),
                    AsPascalCase(&variant.constructor_name)
                ));
                plan.variant_decoders
                    .insert((base.name.clone(), variant.constructor_name.clone()), name);
            }
        }
    }
    plan
}
