use dust_diagnostics::Diagnostic;
use dust_ir::DartFileIr;
use dust_plugin_api::{
    AnnotationOrigin, AuxiliaryOutputContribution, DustPlugin, PluginContext, PluginContribution,
    SymbolPlan, TypeOriginRequest, WorkspaceAnalysisBuilder,
};
use std::path::PathBuf;

use crate::{
    analysis::{
        PayloadCapabilityIndex, collect_payload_model_workspace_analysis_ir,
        collect_query_model_workspace_analysis_ir, collect_server_workspace_analysis_ir,
        hydrate_payload_models, hydrate_query_models,
    },
    client::generate_server_client,
    emit::emit_server,
    plan::{plan_server, plan_server_with_type_origins},
};

/// Annotation ownership and discovery registration.
mod registration;
/// Requests and canonical checks for typed socket declaration origins.
mod type_origins;

use registration::{CLAIMED_CONFIGS, SUPPORTED_ANNOTATIONS};
pub(crate) use type_origins::{CONNECTION_LIBRARY, CONNECTION_NAME, type_origin_request};

/// Dust plugin for Dart HTTP servers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerClientDestination {
    /// Name from the destination package's pubspec.
    pub package_name: String,
    /// Resolved local package root.
    pub package_root: PathBuf,
    /// Route tags accepted by this destination; empty accepts every route.
    pub tags: Vec<String>,
}

/// Dust plugin for Dart HTTP servers.
#[derive(Default)]
pub struct ServerPlugin {
    /// Local output packages for backend-derived clients.
    clients: Vec<ServerClientDestination>,
}

impl ServerPlugin {
    /// Creates a server plugin.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the configured backend-derived client destinations.
    pub fn client_destinations(&self) -> &[ServerClientDestination] {
        &self.clients
    }
}

/// Creates the server plugin.
pub fn register_plugin() -> ServerPlugin {
    ServerPlugin::new()
}

/// Creates the server plugin with generated-client destinations.
pub fn register_plugin_with_clients(clients: Vec<ServerClientDestination>) -> ServerPlugin {
    ServerPlugin { clients }
}

impl DustPlugin for ServerPlugin {
    fn plugin_name(&self) -> &'static str {
        "Server"
    }

    fn claimed_configs(&self) -> &'static [&'static str] {
        CLAIMED_CONFIGS
    }

    fn annotation_origins(&self) -> &'static [AnnotationOrigin] {
        registration::ORIGINS
    }

    fn partless_configs(&self) -> &'static [&'static str] {
        CLAIMED_CONFIGS
    }

    fn supported_annotations(&self) -> &'static [&'static str] {
        SUPPORTED_ANNOTATIONS
    }

    fn type_origin_requests(&self, file: &DartFileIr) -> Vec<TypeOriginRequest> {
        type_origins::requests(file)
    }

    fn collect_workspace_analysis_ir(
        &self,
        file: &DartFileIr,
        analysis: &mut WorkspaceAnalysisBuilder,
    ) {
        collect_server_workspace_analysis_ir(file, analysis, &self.clients);
    }

    fn collect_dependency_analysis_ir(
        &self,
        file: &DartFileIr,
        analysis: &mut WorkspaceAnalysisBuilder,
    ) {
        collect_query_model_workspace_analysis_ir(file, analysis);
        collect_payload_model_workspace_analysis_ir(file, analysis);
    }

    fn validate(&self, file: &DartFileIr) -> Vec<Diagnostic> {
        plan_server(file).err().unwrap_or_default()
    }

    fn validate_with_plan(&self, file: &DartFileIr, symbols: &SymbolPlan) -> Vec<Diagnostic> {
        let analysis = symbols.workspace_analysis();
        match plan_server_with_type_origins(file, &|param| {
            type_origin_request(param).is_some_and(|request| {
                analysis.has_type_origin(
                    &file.source_path,
                    &request,
                    CONNECTION_LIBRARY,
                    CONNECTION_NAME,
                )
            })
        }) {
            Ok(Some(mut plan)) => {
                let models =
                    match PayloadCapabilityIndex::from_analysis(symbols.workspace_analysis()) {
                        Ok(models) => models,
                        Err(error) => return vec![error],
                    };
                let mut diagnostics = hydrate_query_models(
                    file,
                    &mut plan,
                    symbols.workspace_analysis(),
                    !self.clients.is_empty(),
                );
                diagnostics.extend(hydrate_payload_models(
                    file,
                    &mut plan,
                    &models,
                    !self.clients.is_empty(),
                ));
                diagnostics
            }
            Ok(None) => Vec::new(),
            Err(diagnostics) => diagnostics,
        }
    }

    fn generate(&self, file: &DartFileIr, context: &PluginContext<'_>) -> Vec<PluginContribution> {
        let analysis = context.symbol_plan.workspace_analysis();
        match plan_server_with_type_origins(file, &|param| {
            type_origin_request(param).is_some_and(|request| {
                analysis.has_type_origin(
                    &file.source_path,
                    &request,
                    CONNECTION_LIBRARY,
                    CONNECTION_NAME,
                )
            })
        }) {
            Ok(Some(mut plan)) => {
                let models = match PayloadCapabilityIndex::from_analysis(
                    context.symbol_plan.workspace_analysis(),
                ) {
                    Ok(models) => models,
                    Err(error) => {
                        return vec![PluginContribution {
                            diagnostics: vec![error],
                            ..PluginContribution::default()
                        }];
                    }
                };
                let mut diagnostics = hydrate_query_models(
                    file,
                    &mut plan,
                    context.symbol_plan.workspace_analysis(),
                    !self.clients.is_empty(),
                );
                diagnostics.extend(hydrate_payload_models(
                    file,
                    &mut plan,
                    &models,
                    !self.clients.is_empty(),
                ));
                if !diagnostics.is_empty() {
                    return vec![PluginContribution {
                        diagnostics,
                        ..PluginContribution::default()
                    }];
                }
                let mut contribution = emit_server(&plan);
                for destination in &self.clients {
                    match generate_server_client(
                        file,
                        &plan,
                        destination,
                        context.symbol_plan.workspace_analysis(),
                        &models,
                    ) {
                        Ok(outputs) => {
                            contribution
                                .auxiliary_outputs
                                .extend(outputs.into_iter().map(|(output_path, source)| {
                                    AuxiliaryOutputContribution {
                                        output_path,
                                        source,
                                    }
                                }))
                        }
                        Err(diagnostics) => contribution.diagnostics.extend(diagnostics),
                    }
                }
                vec![contribution]
            }
            Ok(None) => vec![PluginContribution::default()],
            Err(diagnostics) => vec![PluginContribution {
                diagnostics,
                ..PluginContribution::default()
            }],
        }
    }
}
