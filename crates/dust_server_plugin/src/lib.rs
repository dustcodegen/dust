#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = "Dust plugin for generated Dart HTTP server wiring."]

/// Structured workspace facts for future documentation tooling.
mod analysis;
/// Backend-route to standalone-client normalization.
mod client;
/// Dart source emission for validated server plans.
mod emit;
/// Phase-one server normalization and validation.
mod plan;
/// Server plugin implementation and registration surface.
mod plugin;

pub use plugin::{
    ServerClientDestination, ServerPlugin, register_plugin, register_plugin_with_clients,
};
