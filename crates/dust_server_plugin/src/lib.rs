#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = "Dust plugin for generated Dart HTTP server wiring."]

/// Server plugin implementation and registration surface.
mod plugin;

pub use plugin::{ServerPlugin, register_plugin};
