#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = "Dust plugin for generated Dart HTTP server wiring."]

/// Dart source emission for validated server plans.
mod emit;
/// Phase-one server normalization and validation.
mod plan;
/// Server plugin implementation and registration surface.
mod plugin;

pub use plugin::{ServerPlugin, register_plugin};
