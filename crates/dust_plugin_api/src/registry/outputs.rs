use std::path::Path;

use dust_diagnostics::Diagnostic;

use crate::{GeneratedLibrary, PluginRegistry};

impl PluginRegistry {
    /// Plans outputs for active and symbols-only plugins without executing them.
    pub fn potential_generated_libraries(
        &self,
        source: &Path,
    ) -> Result<Vec<GeneratedLibrary>, Diagnostic> {
        let mut outputs = Vec::new();
        for registration in &self.plugins {
            for output in registration.plugin.potential_generated_libraries(source) {
                if !registration.config_symbols.contains(&output.producer) {
                    // Fallback claims shadowed by an active plugin cannot grant ownership.
                    if registration
                        .plugin
                        .claimed_configs()
                        .contains(&output.producer.0.as_str())
                    {
                        continue;
                    }
                    return Err(Diagnostic::error(format!(
                        "plugin `{}` declares an output for unclaimed producer `{}`",
                        registration.plugin.plugin_name(),
                        output.producer.0
                    )));
                }
                if !output.path.is_absolute() {
                    return Err(Diagnostic::error(format!(
                        "plugin `{}` declares a non-absolute generated library path",
                        registration.plugin.plugin_name()
                    )));
                }
                outputs.push(output);
            }
        }
        Ok(outputs)
    }
}
