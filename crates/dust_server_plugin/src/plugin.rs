use dust_diagnostics::Diagnostic;
use dust_ir::DartFileIr;
use dust_plugin_api::{
    AnnotationOrigin, DustPlugin, PluginContext, PluginContribution, WorkspaceAnalysisBuilder,
};

use crate::{analysis::collect_server_workspace_analysis_ir, emit::emit_server, plan::plan_server};

/// Server configuration symbols owned by this plugin.
const CLAIMED_CONFIGS: &[&str] = &[
    "dust_server::Controller",
    "dust_server::Routes",
    "dust_server::GET",
    "dust_server::POST",
    "dust_server::PUT",
    "dust_server::PATCH",
    "dust_server::DELETE",
    "dust_server::HEAD",
    "dust_server::OPTIONS",
    "dust_server::Extract",
    "dust_server::Path",
    "dust_server::Query",
    "dust_server::Queries",
    "dust_server::RawQuery",
    "dust_server::Header",
    "dust_server::HeaderMap",
    "dust_server::Body",
    "dust_server::Field",
    "dust_server::Part",
    "dust_server::RawBody",
    "dust_server::State",
    "dust_server::Peer",
    "dust_server::FormUrlEncoded",
    "dust_server::MultiPart",
];

/// Surface names used by workspace discovery.
const SUPPORTED_ANNOTATIONS: &[&str] = &[
    "Controller",
    "Routes",
    "GET",
    "POST",
    "PUT",
    "PATCH",
    "DELETE",
    "HEAD",
    "OPTIONS",
    "Extract",
    "Path",
    "Query",
    "Queries",
    "RawQuery",
    "Header",
    "HeaderMap",
    "Body",
    "Field",
    "Part",
    "RawBody",
    "State",
    "Peer",
    "FormUrlEncoded",
    "MultiPart",
];

/// Declaring library for `Controller`.
const CONTROLLER_LIBRARY: &str = "package:dust_server/src/annotations/controller.dart";
/// Declaring library for `Routes`.
const ROUTES_LIBRARY: &str = "package:dust_server/src/annotations/routes.dart";
/// Declaring library for HTTP method annotations.
const METHODS_LIBRARY: &str = "package:dust_server/src/annotations/http_methods.dart";
/// Declaring library for parameter and body-mode annotations.
const PARAMETERS_LIBRARY: &str = "package:dust_server/src/annotations/parameters.dart";

/// Dust plugin for Dart HTTP servers.
#[derive(Default)]
pub struct ServerPlugin;

impl ServerPlugin {
    /// Creates a server plugin.
    pub fn new() -> Self {
        Self
    }
}

/// Creates the server plugin.
pub fn register_plugin() -> ServerPlugin {
    ServerPlugin::new()
}

impl DustPlugin for ServerPlugin {
    fn plugin_name(&self) -> &'static str {
        "Server"
    }

    fn claimed_configs(&self) -> &'static [&'static str] {
        CLAIMED_CONFIGS
    }

    fn annotation_origins(&self) -> &'static [AnnotationOrigin] {
        const {
            &[
                AnnotationOrigin::new("dust_server::Controller", CONTROLLER_LIBRARY, "Controller"),
                AnnotationOrigin::new("dust_server::Routes", ROUTES_LIBRARY, "Routes"),
                AnnotationOrigin::new("dust_server::GET", METHODS_LIBRARY, "GET"),
                AnnotationOrigin::new("dust_server::POST", METHODS_LIBRARY, "POST"),
                AnnotationOrigin::new("dust_server::PUT", METHODS_LIBRARY, "PUT"),
                AnnotationOrigin::new("dust_server::PATCH", METHODS_LIBRARY, "PATCH"),
                AnnotationOrigin::new("dust_server::DELETE", METHODS_LIBRARY, "DELETE"),
                AnnotationOrigin::new("dust_server::HEAD", METHODS_LIBRARY, "HEAD"),
                AnnotationOrigin::new("dust_server::OPTIONS", METHODS_LIBRARY, "OPTIONS"),
                AnnotationOrigin::new("dust_server::Extract", PARAMETERS_LIBRARY, "Extract"),
                AnnotationOrigin::new("dust_server::Path", PARAMETERS_LIBRARY, "Path"),
                AnnotationOrigin::new("dust_server::Query", PARAMETERS_LIBRARY, "Query"),
                AnnotationOrigin::new("dust_server::Queries", PARAMETERS_LIBRARY, "Queries"),
                AnnotationOrigin::new("dust_server::RawQuery", PARAMETERS_LIBRARY, "RawQuery"),
                AnnotationOrigin::new("dust_server::Header", PARAMETERS_LIBRARY, "Header"),
                AnnotationOrigin::new("dust_server::HeaderMap", PARAMETERS_LIBRARY, "HeaderMap"),
                AnnotationOrigin::new("dust_server::Body", PARAMETERS_LIBRARY, "Body"),
                AnnotationOrigin::new("dust_server::Field", PARAMETERS_LIBRARY, "Field"),
                AnnotationOrigin::new("dust_server::Part", PARAMETERS_LIBRARY, "Part"),
                AnnotationOrigin::new("dust_server::RawBody", PARAMETERS_LIBRARY, "RawBody"),
                AnnotationOrigin::new("dust_server::State", PARAMETERS_LIBRARY, "State"),
                AnnotationOrigin::new("dust_server::Peer", PARAMETERS_LIBRARY, "Peer"),
                AnnotationOrigin::new(
                    "dust_server::FormUrlEncoded",
                    PARAMETERS_LIBRARY,
                    "FormUrlEncoded",
                ),
                AnnotationOrigin::new("dust_server::MultiPart", PARAMETERS_LIBRARY, "MultiPart"),
            ]
        }
    }

    fn partless_configs(&self) -> &'static [&'static str] {
        CLAIMED_CONFIGS
    }

    fn supported_annotations(&self) -> &'static [&'static str] {
        SUPPORTED_ANNOTATIONS
    }

    fn collect_workspace_analysis_ir(
        &self,
        file: &DartFileIr,
        analysis: &mut WorkspaceAnalysisBuilder,
    ) {
        collect_server_workspace_analysis_ir(file, analysis);
    }

    fn validate(&self, file: &DartFileIr) -> Vec<Diagnostic> {
        plan_server(file).err().unwrap_or_default()
    }

    fn generate(&self, file: &DartFileIr, _context: &PluginContext<'_>) -> Vec<PluginContribution> {
        match plan_server(file) {
            Ok(Some(plan)) => vec![emit_server(&plan)],
            Ok(None) => vec![PluginContribution::default()],
            Err(diagnostics) => vec![PluginContribution {
                diagnostics,
                ..PluginContribution::default()
            }],
        }
    }
}
