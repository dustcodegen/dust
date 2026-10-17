#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = "Built-in Dust plugin providing JSON serialization and deserialization support for Dart classes and enums."]

/// Workspace analysis for serde validation.
mod analysis;
/// Coordinates serde code emission for a library.
mod emit;
/// Renders class JSON helpers and mixins.
mod emit_class;
/// Renders enum JSON helpers.
mod emit_enum;
/// Renders sealed class JSON dispatch helpers.
mod emit_sealed;
/// Shared formatting helpers for serde emission.
mod emit_support;
/// Renders generated sealed variant classes.
mod emit_variant_class;
/// How a use site reaches an enum's JSON helpers, in its library or another.
mod enum_codecs;
/// Scoped generated JSON references and local names.
mod names;
/// Plugin registration and Dust plugin implementation.
mod plugin;
/// Validates serde-compatible model shapes.
mod validate;
/// Constructor-specific sealed decoding without changing ordinary leaf codecs.
mod variant_decoders;
/// Structured sealed target applications and binder ownership.
mod variant_types;
/// Low-level helpers for generating JSON-related Dart expressions.
pub mod writer;
/// Renders JSON encode and decode expressions.
mod writer_expr;
/// Renders model-level constructor and key helpers.
mod writer_model;
/// Renders type and receiver helpers.
mod writer_type;

pub use plugin::{SerdePlugin, register_plugin};
