use dust_diagnostics::Diagnostic;
use dust_ir::DartFileIr;
use dust_plugin_api::{
    AnnotationOrigin, DustPlugin, PluginContext, PluginContribution, SymbolPlan,
};

use crate::{
    emit::emit_library,
    features::{COPY_WITH_SYMBOL, DEBUG_SYMBOL, EQ_SYMBOL, TO_STRING_SYMBOL, VALIDATE_SYMBOL},
    validate::{validate_library, validate_library_with_plan},
};

/// The built-in plugin that implements Dust's core derive traits.
pub struct DerivePlugin;

/// Creates the built-in derive plugin instance.
pub fn register_plugin() -> DerivePlugin {
    DerivePlugin
}

/// Trait symbols claimed by the derive plugin.
pub(crate) const CLAIMED_TRAITS: &[&str] = &[
    TO_STRING_SYMBOL,
    DEBUG_SYMBOL,
    EQ_SYMBOL,
    COPY_WITH_SYMBOL,
    VALIDATE_SYMBOL,
];

/// Config symbols claimed by the derive plugin.
const CLAIMED_CONFIGS: &[&str] = &[VALIDATE_SYMBOL];

/// Short annotation names supported by the derive plugin.
const SUPPORTED_ANNOTATIONS: &[&str] =
    &["Derive", "ToString", "Debug", "Eq", "CopyWith", "Validate"];

/// Actual declarations, not the public derive barrel.
const TRAIT_LIBRARY: &str = "package:dust_dart/src/derive/traits.dart";

impl DustPlugin for DerivePlugin {
    fn plugin_name(&self) -> &'static str {
        "dust_plugin_derive"
    }

    fn claimed_traits(&self) -> &'static [&'static str] {
        CLAIMED_TRAITS
    }

    fn claimed_configs(&self) -> &'static [&'static str] {
        CLAIMED_CONFIGS
    }

    fn annotation_origins(&self) -> &'static [AnnotationOrigin] {
        const {
            &[
                AnnotationOrigin::new(TO_STRING_SYMBOL, TRAIT_LIBRARY, "ToString"),
                AnnotationOrigin::new(DEBUG_SYMBOL, TRAIT_LIBRARY, "Debug"),
                AnnotationOrigin::new(EQ_SYMBOL, TRAIT_LIBRARY, "Eq"),
                AnnotationOrigin::new(COPY_WITH_SYMBOL, TRAIT_LIBRARY, "CopyWith"),
                AnnotationOrigin::new(
                    VALIDATE_SYMBOL,
                    "package:dust_dart/src/derive/validation.dart",
                    "Validate",
                ),
            ]
        }
    }

    fn supported_annotations(&self) -> &'static [&'static str] {
        SUPPORTED_ANNOTATIONS
    }

    fn requested_symbols(&self, _library: &DartFileIr) -> Vec<String> {
        Vec::new()
    }

    fn validate(&self, library: &DartFileIr) -> Vec<Diagnostic> {
        validate_library(library)
    }

    fn validate_with_plan(&self, library: &DartFileIr, plan: &SymbolPlan) -> Vec<Diagnostic> {
        validate_library_with_plan(library, plan)
    }

    fn generate(
        &self,
        library: &DartFileIr,
        context: &PluginContext<'_>,
    ) -> Vec<PluginContribution> {
        vec![emit_library(library, context.symbol_plan)]
    }
}
