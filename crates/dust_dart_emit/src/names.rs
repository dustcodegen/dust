use std::collections::BTreeSet;

use dust_ir::DartFileIr;

/// Deterministic allocator for generated names with numeric suffix fallback.
#[derive(Clone)]
pub struct NameAllocator {
    /// Reserved and already allocated names in this generated scope.
    used: BTreeSet<String>,
}

impl NameAllocator {
    /// Creates an allocator seeded with reserved names.
    pub fn new<I, S>(reserved: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            used: reserved.into_iter().map(Into::into).collect(),
        }
    }

    /// Allocates a unique name based on the requested base.
    pub fn allocate(&mut self, base: impl Into<String>) -> String {
        let base = base.into();
        if self.used.insert(base.clone()) {
            return base;
        }
        (2_usize..)
            .find_map(|suffix| {
                let candidate = format!("{base}{suffix}");
                self.used.insert(candidate.clone()).then_some(candidate)
            })
            .expect("finite reserved names leave an unused suffix")
    }
}

/// Collects top-level declaration names that generated code must avoid.
pub fn library_declaration_names(library: &DartFileIr) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    names.extend(library.classes.iter().map(|class| class.name.clone()));
    names.extend(library.mixins.iter().map(|mixin| mixin.name.source.clone()));
    names.extend(
        library
            .extensions
            .iter()
            .filter_map(|extension| extension.name.as_ref())
            .map(|name| name.source.clone()),
    );
    names.extend(
        library
            .extension_types
            .iter()
            .map(|extension| extension.name.source.clone()),
    );
    names.extend(
        library
            .functions
            .iter()
            .map(|function| function.name.source.clone()),
    );
    names.extend(
        library
            .variables
            .iter()
            .map(|variable| variable.name.source.clone()),
    );
    names.extend(
        library
            .typedefs
            .iter()
            .map(|typedef| typedef.name.source.clone()),
    );
    names.extend(library.enums.iter().map(|item| item.name.clone()));
    names
}

/// Reserves names that could capture a generated library reference inside a class.
pub fn library_reference_names(library: &DartFileIr) -> BTreeSet<String> {
    let mut names = library_declaration_names(library);
    for class in &library.classes {
        names.extend(class.type_parameters.iter().map(|p| p.name.source.clone()));
        names.extend(class.fields.iter().map(|field| field.name.clone()));
        names.extend(class.methods.iter().map(|method| method.name.clone()));
    }
    names
}
