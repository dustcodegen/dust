use dust_diagnostics::Diagnostic;
use dust_ir::DartFileIr;
use dust_plugin_api::{
    AnnotationOrigin, AuxiliaryOutputContribution, DustPlugin, PluginContext, PluginContribution,
    SymbolPlan, WorkspaceAnalysisBuilder,
};
use dust_workspace::generated_test_output_path;

/// Converts parsed IR into validated HTTP client specs.
mod build;
/// Shared annotation and configuration symbol names.
mod constants;
/// Renders generated Dart HTTP client sources.
mod emit;
/// Internal HTTP client model used after validation.
mod model;
/// Extracts HTTP annotations from lowered Dart IR.
mod parse;
/// Shared helpers for HTTP plugin parsing, validation, and rendering.
mod util;
/// Validates annotated HTTP client classes before emission.
mod validate;

use self::build::build_client_spec;
use self::constants::{CLAIMED_CONFIG_SYMBOLS, HTTP_CLIENT, SUPPORTED_ANNOTATIONS};
use self::emit::{
    render_client_class, render_isolate_helpers, render_shared_helpers, render_test_file,
};
use self::parse::has_config_named;
use self::validate::{JsonCapabilityContext, collect_workspace_analysis_ir, validate_client_class};

/// Dust plugin for generating Dio-backed HTTP clients.
pub struct HttpClientPlugin;

/// HTTP Query is not the separate SQL Query declared in DB annotations.
const ANNOTATION_LIBRARY: &str = "package:dust_dart/src/http/http_client_annotations.dart";

impl HttpClientPlugin {
    /// Creates a new instance of the plugin.
    pub fn new() -> Self {
        Self
    }
}

impl Default for HttpClientPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates the HttpClient plugin.
pub fn register_plugin() -> HttpClientPlugin {
    HttpClientPlugin::new()
}

impl DustPlugin for HttpClientPlugin {
    fn plugin_name(&self) -> &'static str {
        "HttpClient"
    }

    fn claimed_configs(&self) -> &'static [&'static str] {
        CLAIMED_CONFIG_SYMBOLS
    }

    fn annotation_origins(&self) -> &'static [AnnotationOrigin] {
        const {
            &[
                AnnotationOrigin::new("dust_dart::HttpClient", ANNOTATION_LIBRARY, "HttpClient"),
                AnnotationOrigin::new("dust_dart::GET", ANNOTATION_LIBRARY, "GET"),
                AnnotationOrigin::new("dust_dart::POST", ANNOTATION_LIBRARY, "POST"),
                AnnotationOrigin::new("dust_dart::PUT", ANNOTATION_LIBRARY, "PUT"),
                AnnotationOrigin::new("dust_dart::PATCH", ANNOTATION_LIBRARY, "PATCH"),
                AnnotationOrigin::new("dust_dart::DELETE", ANNOTATION_LIBRARY, "DELETE"),
                AnnotationOrigin::new("dust_dart::HEAD", ANNOTATION_LIBRARY, "HEAD"),
                AnnotationOrigin::new("dust_dart::OPTIONS", ANNOTATION_LIBRARY, "OPTIONS"),
                AnnotationOrigin::new("dust_dart::Path", ANNOTATION_LIBRARY, "Path"),
                AnnotationOrigin::new("dust_dart::Query", ANNOTATION_LIBRARY, "Query"),
                AnnotationOrigin::new("dust_dart::Queries", ANNOTATION_LIBRARY, "Queries"),
                AnnotationOrigin::new("dust_dart::Header", ANNOTATION_LIBRARY, "Header"),
                AnnotationOrigin::new("dust_dart::Headers", ANNOTATION_LIBRARY, "Headers"),
                AnnotationOrigin::new("dust_dart::HeaderMap", ANNOTATION_LIBRARY, "HeaderMap"),
                AnnotationOrigin::new("dust_dart::Body", ANNOTATION_LIBRARY, "Body"),
                AnnotationOrigin::new("dust_dart::Field", ANNOTATION_LIBRARY, "Field"),
                AnnotationOrigin::new("dust_dart::Part", ANNOTATION_LIBRARY, "Part"),
                AnnotationOrigin::new("dust_dart::Extra", ANNOTATION_LIBRARY, "Extra"),
                AnnotationOrigin::new(
                    "dust_dart::FormUrlEncoded",
                    ANNOTATION_LIBRARY,
                    "FormUrlEncoded",
                ),
                AnnotationOrigin::new("dust_dart::MultiPart", ANNOTATION_LIBRARY, "MultiPart"),
                AnnotationOrigin::new("dust_dart::HttpParse", ANNOTATION_LIBRARY, "HttpParse"),
            ]
        }
    }

    fn supported_annotations(&self) -> &'static [&'static str] {
        SUPPORTED_ANNOTATIONS
    }

    fn collect_workspace_analysis_ir(
        &self,
        library: &DartFileIr,
        analysis: &mut WorkspaceAnalysisBuilder,
    ) {
        collect_workspace_analysis_ir(library, analysis);
    }

    fn validate(&self, library: &DartFileIr) -> Vec<Diagnostic> {
        let json = JsonCapabilityContext::new(library);
        library
            .classes
            .iter()
            .flat_map(|class| validate_client_class(&library.imports, class, Some(&json)))
            .collect()
    }

    fn validate_with_plan(&self, library: &DartFileIr, plan: &SymbolPlan) -> Vec<Diagnostic> {
        let json = JsonCapabilityContext::with_workspace(library, Some(plan.workspace_analysis()));
        library
            .classes
            .iter()
            .flat_map(|class| validate_client_class(&library.imports, class, Some(&json)))
            .collect()
    }

    fn generate(
        &self,
        library: &DartFileIr,
        _context: &PluginContext<'_>,
    ) -> Vec<PluginContribution> {
        let mut contribution = PluginContribution::default();
        let mut generated_any = false;
        let mut generated_test_specs = Vec::new();

        for class in &library.classes {
            if !has_config_named(&class.configs, HTTP_CLIENT) {
                continue;
            }

            let spec = match build_client_spec(&library.imports, class) {
                Ok(spec) => spec,
                Err(_) => continue,
            };

            generated_any = true;
            contribution.support_types.push(render_client_class(&spec));
            contribution
                .top_level_functions
                .extend(render_isolate_helpers(&spec));
            if spec.generate_test {
                generated_test_specs.push(spec);
            }
        }

        if generated_any {
            contribution.shared_helpers.extend(render_shared_helpers());
        }
        if !generated_test_specs.is_empty() {
            if let Some(source) = render_test_file(library, &generated_test_specs) {
                if let Ok(output_path) = generated_test_output_path(
                    std::path::Path::new(&library.package_root),
                    std::path::Path::new(&library.source_path),
                ) {
                    contribution
                        .auxiliary_outputs
                        .push(AuxiliaryOutputContribution {
                            output_path,
                            source,
                        });
                }
            }
        }

        vec![contribution]
    }
}
